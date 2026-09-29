use crate::app::{App, AppEvent, InputMode, Mode};
use crate::infra::channel::{self, Receiver};
use crate::infra::terminal::TerminalSessionGuard;
use crate::preview::{self, structured_log};
use crate::preview::{PreviewRequest, PreviewSource};
use crate::runtime::interactive::handlers;
use crate::runtime::interactive::input::InputEvent;
use crate::search::controller::SearchController;
use crate::search::matcher::MatcherCommand;
use crate::ui;
use crossterm::{
    execute, queue,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

type InteractiveTerminal = Terminal<CrosstermBackend<io::BufWriter<io::Stderr>>>;

const MAX_PENDING_EVENTS_PER_FRAME: usize = 64;
const PERIODIC_RENDER_INTERVAL: Duration = Duration::from_millis(120);

pub fn run_event_loop(
    app: &mut App,
    terminal: &mut InteractiveTerminal,
    terminal_session: &mut TerminalSessionGuard,
    rx_main: &channel::DefaultReceiver<AppEvent>,
    tx_preview_req: &channel::DefaultSender<PreviewRequest>,
    tx_cmd_noop: &channel::DefaultSender<MatcherCommand>,
    search_sessions: &mut Option<SearchController>,
    log_max_entries: usize,
) -> anyhow::Result<()> {
    let mut item_limit = 100;

    render_frame(app, terminal, terminal_session)?;
    let mut last_render_at = Instant::now();

    loop {
        let event = match rx_main.recv() {
            Ok(event) => event,
            Err(channel::ChannelError::Disconnected) => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        // Start after recv: idle waiting is not UI processing time.
        let batch_started = app.ui.bench.as_ref().map(|_| Instant::now());
        let mut saw_tick = matches!(event, AppEvent::Input(InputEvent::Tick));
        let mut render_requested = handle_app_event(
            app,
            event,
            tx_preview_req,
            tx_cmd_noop,
            search_sessions,
            log_max_entries,
        );

        for _ in 0..MAX_PENDING_EVENTS_PER_FRAME {
            let event = match rx_main.try_recv() {
                Ok(Some(event)) => event,
                Ok(None) | Err(channel::ChannelError::Disconnected) => break,
                Err(err) => return Err(err.into()),
            };
            saw_tick |= matches!(event, AppEvent::Input(InputEvent::Tick));
            render_requested |= handle_app_event(
                app,
                event,
                tx_preview_req,
                tx_cmd_noop,
                search_sessions,
                log_max_entries,
            );
        }

        if let Some(search_sessions) = search_sessions.as_mut() {
            search_sessions.reconcile(app, &mut item_limit);
        }

        if let Some(search_sessions) = search_sessions.as_ref() {
            if let Some(tx_cmd) = search_sessions.command_sender() {
                handlers::check_infinite_scroll(app, &mut item_limit, tx_cmd);
            }
        }

        if let (Some(bench), Some(started)) = (app.ui.bench.as_mut(), batch_started) {
            if render_requested {
                bench.event_batches.record(started.elapsed());
            }
        }

        if app.ui.should_quit {
            return Ok(());
        }

        if saw_tick
            && !render_requested
            && should_render_periodically(app)
            && last_render_at.elapsed() >= PERIODIC_RENDER_INTERVAL
        {
            render_requested = true;
        }

        if render_requested {
            render_frame(app, terminal, terminal_session)?;
            last_render_at = Instant::now();
        }
    }
}

fn render_frame(
    app: &mut App,
    terminal: &mut InteractiveTerminal,
    terminal_session: &mut TerminalSessionGuard,
) -> anyhow::Result<()> {
    let draw_started = app.ui.bench.as_ref().map(|_| Instant::now());
    app.refresh_viewports();
    sync_cursor_style(app, terminal_session);

    queue!(terminal.backend_mut(), BeginSynchronizedUpdate).ok();
    terminal.draw(|f| ui::draw(f, app))?;
    if app.ui.bench.as_ref().is_some_and(|bench| bench.visible) {
        terminal.hide_cursor()?;
    }
    execute!(terminal.backend_mut(), EndSynchronizedUpdate).ok();

    if let (Some(bench), Some(started)) = (app.ui.bench.as_mut(), draw_started) {
        bench.record_draw(started.elapsed());
    }

    Ok(())
}

fn should_render_periodically(app: &App) -> bool {
    app.ui.bench.as_ref().is_some_and(|bench| bench.visible)
        || app.search_session.search.working
        || app
            .preview_session
            .preview
            .state
            .status_message
            .as_ref()
            .is_some_and(|(_, shown_at)| shown_at.elapsed().as_secs() < 3)
}

fn sync_cursor_style(app: &App, terminal_session: &mut TerminalSessionGuard) {
    let should_be_bar =
        app.ui.mode == Mode::Preview && app.preview_session.preview.state.mode == InputMode::Insert;
    terminal_session.sync_cursor_style(should_be_bar);
}

fn handle_app_event(
    app: &mut App,
    event: AppEvent,
    tx_preview_req: &channel::DefaultSender<PreviewRequest>,
    tx_cmd_noop: &channel::DefaultSender<MatcherCommand>,
    search_sessions: &Option<SearchController>,
    log_max_entries: usize,
) -> bool {
    match event {
        AppEvent::Input(input) => match input {
            InputEvent::Key(key) => {
                let tx_cmd = search_sessions
                    .as_ref()
                    .and_then(SearchController::command_sender)
                    .unwrap_or(tx_cmd_noop);
                handlers::handle_input(app, key, tx_cmd, tx_preview_req);
                true
            }
            InputEvent::Resize(width, height) => {
                app.set_terminal_size(width, height);
                app.refresh_viewports();
                true
            }
            InputEvent::Tick => false,
        },
        AppEvent::Matcher(state, epoch) => {
            if search_sessions
                .as_ref()
                .is_some_and(|search_sessions| search_sessions.accepts_epoch(epoch))
            {
                apply_matcher_state(app, state, tx_preview_req);
                true
            } else {
                false
            }
        }
        AppEvent::Preview(source, text) => {
            apply_preview_event(app, source, text);
            true
        }
        AppEvent::LogAppend(path, entries) => {
            structured_log::apply_append(app, &path, entries, log_max_entries);
            true
        }
    }
}

fn apply_matcher_state(
    app: &mut App,
    mut state: crate::search::matcher::MatcherState,
    tx_preview: &channel::DefaultSender<PreviewRequest>,
) {
    let search = &app.search_session.search;
    let selection_preserved = search.selection < state.results_start
        && search
            .results
            .get(search.selection)
            .is_some_and(|result| search.selected_item.as_ref() == Some(&result.item));
    state.apply_results(&mut app.search_session.search.results);
    if !app.search_session.search.results.is_empty() {
        if let Some(bench) = app.ui.bench.as_mut() {
            bench
                .first_results
                .get_or_insert_with(|| bench.started_at.elapsed());
        }
    }
    app.search_session.search.total_matches = state.total_matches;
    app.search_session.search.total_items = state.total_items;
    app.search_session.search.working = state.working;
    if !selection_preserved {
        app.search_session.search.update_selection();
        handlers::sync_preview(app, tx_preview);
    }
}

pub fn apply_preview_event(app: &mut App, source: PreviewSource, text: preview::PreviewContent) {
    if app.preview_session.preview.source.as_ref() != Some(&source) {
        return;
    }

    if let Some(bench) = app.ui.bench.as_mut() {
        bench
            .first_preview
            .get_or_insert_with(|| bench.started_at.elapsed());
    }

    let preview_is_rich_text = matches!(text, preview::PreviewContent::RichText(_))
        && !matches!(source, PreviewSource::GitHistory { .. });
    let preview_is_log = matches!(text, preview::PreviewContent::StructuredLog(_));
    app.preview_session.preview.content = Some(text);

    if app.ui.mode == Mode::Preview && !preview_is_rich_text && !preview_is_log {
        app.ui.mode = Mode::Search;
        app.preview_session.preview.state.search_active = false;
        app.preview_session.preview.state.status_message = Some((
            "Preview is read-only for this file type".to_string(),
            std::time::Instant::now(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoadedAppConfig;
    use crate::infra::channel::{unbounded_default, Receiver};
    use crate::runtime::config::RunConfig;
    use crate::search::sources::git::{GitSearchMode, GitSearchScope};
    use crate::search::types::{
        MatcherMode, SearchConfig, SearchItem, SearchMode, SearchResult, SearchSettings,
    };
    use std::path::{Path, PathBuf};

    fn run_config() -> RunConfig {
        RunConfig {
            headless: false,
            bench: false,
            output_format: crate::cli::args::OutputFormat::Plain,
            output_file: None,
            stdin: false,
            log: false,
            diff: None,
            preview_command: None,
            preview_delimiter: ":".to_string(),
            split: None,
            log_files: Vec::new(),
        }
    }

    fn search_config() -> SearchConfig {
        SearchConfig {
            query: None,
            locations: vec![],
            search_pdf: false,
            no_hidden: false,
            no_git_ignore: false,
            no_ignore: false,
            no_default_ignore_dirs: false,
            git_search_scope: None,
            settings: SearchSettings {
                mode: SearchMode::Path,
                matcher: MatcherMode::Fuzzy,
            },
        }
    }

    fn app() -> App {
        App::from_configs(run_config(), search_config(), LoadedAppConfig::default())
    }

    #[test]
    fn bench_refreshes_only_while_visible() {
        let mut app = app();
        assert!(!should_render_periodically(&app));
        app.ui.bench = Some(crate::app::bench::BenchState::new(Instant::now()));
        assert!(should_render_periodically(&app));
        app.apply_action(crate::app::AppAction::CloseBench);
        assert!(!should_render_periodically(&app));
    }

    #[test]
    fn preview_milestone_ignores_stale_content_and_preserves_first_load() {
        let mut app = app();
        app.ui.bench = Some(crate::app::bench::BenchState::new(Instant::now()));
        let current = PreviewSource::SearchItem(SearchItem::path("current.txt"));
        let stale = PreviewSource::SearchItem(SearchItem::path("stale.txt"));
        app.preview_session.preview.source = Some(current.clone());
        apply_preview_event(
            &mut app,
            stale,
            preview::PreviewContent::PlainText("old".into()),
        );
        assert!(app.ui.bench.as_ref().unwrap().first_preview.is_none());
        apply_preview_event(
            &mut app,
            current.clone(),
            preview::PreviewContent::PlainText("new".into()),
        );
        let first = app.ui.bench.as_ref().unwrap().first_preview;
        assert!(first.is_some());
        apply_preview_event(
            &mut app,
            current,
            preview::PreviewContent::PlainText("updated".into()),
        );
        assert_eq!(app.ui.bench.as_ref().unwrap().first_preview, first);
    }

    #[test]
    fn periodic_render_runs_while_search_is_working() {
        let mut app = app();
        app.search_session.search.working = true;

        assert!(should_render_periodically(&app));
    }

    #[test]
    fn periodic_render_stops_after_status_message_expires() {
        let mut app = app();
        app.preview_session.preview.state.status_message =
            Some(("Saved".to_string(), Instant::now() - Duration::from_secs(4)));

        assert!(!should_render_periodically(&app));
    }

    #[test]
    fn git_history_rich_text_preview_stays_read_only() {
        let mut search_config = search_config();
        search_config.git_search_scope = Some(GitSearchScope {
            repo_root: PathBuf::from("/repo"),
            mode: GitSearchMode::History {
                file: PathBuf::from("Architecture.md"),
            },
            display_path: Some("Architecture.md".to_string()),
        });
        let mut app = App::from_configs(run_config(), search_config, LoadedAppConfig::default());
        app.ui.mode = Mode::Preview;
        let source = PreviewSource::GitHistory {
            commit: "HEAD".to_string(),
            path: "Architecture.md".to_string(),
            line: 2,
        };
        app.preview_session.preview.source = Some(source.clone());

        apply_preview_event(
            &mut app,
            source,
            preview::PreviewContent::RichText(preview::create_rich_text_document(
                "fn main() {}\n".to_string(),
                Path::new("Architecture.md"),
            )),
        );

        assert_eq!(app.ui.mode, Mode::Search);
        assert!(matches!(
            app.preview_session.preview.content,
            Some(preview::PreviewContent::RichText(_))
        ));
        assert!(app.preview_session.preview.state.status_message.is_some());
    }

    #[test]
    fn matcher_tails_preserve_selection_marks_and_preview() {
        use crate::search::matcher::MatcherState;
        let mut app = app();
        let (tx_preview, rx_preview) = unbounded_default();
        let row = |name| SearchResult {
            item: SearchItem::path(name),
            indices: vec![0],
            column: None,
        };
        apply_matcher_state(
            &mut app,
            MatcherState {
                results_start: 0,
                results: vec![row("a"), row("b")],
                total_matches: 3,
                total_items: 3,
                working: false,
            },
            &tx_preview,
        );
        app.search_session.search.next();
        handlers::sync_preview(&mut app, &tx_preview);
        while rx_preview.try_recv().unwrap().is_some() {}
        let selected = app.search_session.search.selected_item.clone().unwrap();
        app.search_session
            .search
            .marked_items
            .insert(selected.clone(), None);
        let source = app.preview_session.preview.source.clone();
        for (start, rows) in [(2, vec![row("c")]), (3, vec![])] {
            apply_matcher_state(
                &mut app,
                MatcherState {
                    results_start: start,
                    results: rows,
                    total_matches: 3,
                    total_items: 3,
                    working: false,
                },
                &tx_preview,
            );
            assert_eq!(app.search_session.search.selection, 1);
            assert_eq!(
                app.search_session.search.selected_item,
                Some(selected.clone())
            );
            assert!(app
                .search_session
                .search
                .marked_items
                .contains_key(&selected));
            assert_eq!(app.preview_session.preview.source, source);
            assert!(rx_preview.try_recv().unwrap().is_none());
        }
        assert_eq!(app.search_session.search.results.len(), 3);
    }

    #[test]
    fn grep_matcher_state_syncs_preview_request_and_highlight() {
        let mut app = App::from_configs(run_config(), search_config(), LoadedAppConfig::default());
        app.search_session.settings.mode = SearchMode::Grep;
        app.set_terminal_size(120, 40);
        app.refresh_viewports();

        let (tx_preview, rx_preview) = unbounded_default();
        apply_matcher_state(
            &mut app,
            crate::search::matcher::MatcherState {
                results_start: 0,
                results: vec![SearchResult {
                    item: SearchItem::grep("src/main.rs", 24, "fn main()"),
                    indices: vec![],
                    column: Some(4),
                }],
                total_matches: 1,
                total_items: 1,
                working: false,
            },
            &tx_preview,
        );

        let request = rx_preview.recv().expect("preview request");
        assert_eq!(
            request,
            PreviewRequest::Grep {
                source: PreviewSource::SearchItem(SearchItem::grep("src/main.rs", 24, "fn main()")),
                path: "src/main.rs".to_string(),
                line: 24,
                text: "fn main()".to_string(),
            }
        );
        assert_eq!(app.preview_session.preview.state.highlight_line, Some(24));
    }

    #[test]
    fn git_history_matcher_state_syncs_history_preview_request() {
        let mut search_config = search_config();
        search_config.git_search_scope = Some(GitSearchScope {
            repo_root: Path::new("/repo").to_path_buf(),
            mode: GitSearchMode::History {
                file: Path::new("Architecture.md").to_path_buf(),
            },
            display_path: Some("Architecture.md".to_string()),
        });
        let mut app = App::from_configs(run_config(), search_config, LoadedAppConfig::default());
        app.search_session.settings.mode = SearchMode::GitHistory;
        app.set_terminal_size(120, 40);
        app.refresh_viewports();

        let (tx_preview, rx_preview) = unbounded_default();
        apply_matcher_state(
            &mut app,
            crate::search::matcher::MatcherState {
                results_start: 0,
                results: vec![SearchResult {
                    item: SearchItem::history_line("abc123", "Architecture.md", 24, "fn main()"),
                    indices: vec![],
                    column: Some(4),
                }],
                total_matches: 1,
                total_items: 1,
                working: false,
            },
            &tx_preview,
        );

        let request = rx_preview.recv().expect("preview request");
        assert_eq!(
            request,
            PreviewRequest::GitHistory {
                source: PreviewSource::GitHistory {
                    commit: "abc123".to_string(),
                    path: "Architecture.md".to_string(),
                    line: 24,
                },
                repo_root: "/repo".to_string(),
                commit: "abc123".to_string(),
                path: "Architecture.md".to_string(),
                line: 24,
            }
        );
        assert_eq!(app.preview_session.preview.state.highlight_line, Some(24));
    }
}
