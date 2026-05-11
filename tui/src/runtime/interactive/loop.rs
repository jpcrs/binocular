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
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub fn run_event_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
    terminal_session: &mut TerminalSessionGuard,
    rx_main: &channel::DefaultReceiver<AppEvent>,
    tx_preview_req: &channel::DefaultSender<PreviewRequest>,
    tx_cmd_noop: &channel::DefaultSender<MatcherCommand>,
    search_sessions: &mut Option<SearchController>,
    log_max_entries: usize,
) -> anyhow::Result<()> {
    let mut item_limit = 100;

    loop {
        app.refresh_viewports();
        sync_cursor_style(app, terminal_session);
        terminal.draw(|f| ui::draw(f, app))?;

        if let Ok(event) = rx_main.recv() {
            handle_app_event(
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

        if app.ui.should_quit {
            return Ok(());
        }
    }
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
) {
    match event {
        AppEvent::Input(input) => match input {
            InputEvent::Key(key) => {
                let tx_cmd = search_sessions
                    .as_ref()
                    .and_then(SearchController::command_sender)
                    .unwrap_or(tx_cmd_noop);
                handlers::handle_input(app, key, tx_cmd, tx_preview_req);
            }
            InputEvent::Resize(width, height) => {
                app.set_terminal_size(width, height);
                app.refresh_viewports();
            }
            InputEvent::Tick => {}
        },
        AppEvent::Matcher(state, epoch) => {
            if search_sessions
                .as_ref()
                .is_some_and(|search_sessions| search_sessions.accepts_epoch(epoch))
            {
                apply_matcher_state(app, state, tx_preview_req);
            }
        }
        AppEvent::Preview(source, text) => apply_preview_event(app, source, text),
        AppEvent::LogAppend(path, entries) => {
            structured_log::apply_append(app, &path, entries, log_max_entries);
        }
    }
}

fn apply_matcher_state(
    app: &mut App,
    state: crate::search::matcher::MatcherState,
    tx_preview: &channel::DefaultSender<PreviewRequest>,
) {
    app.search_session.search.results = state.results;
    app.search_session.search.total_matches = state.total_matches;
    app.search_session.search.total_items = state.total_items;
    app.search_session.search.working = state.working;
    app.search_session.search.update_selection();
    handlers::sync_preview(app, tx_preview);
}

pub fn apply_preview_event(app: &mut App, source: PreviewSource, text: preview::PreviewContent) {
    if app.preview_session.preview.source.as_ref() != Some(&source) {
        return;
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
            output_format: crate::cli::args::OutputFormat::Plain,
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
    fn grep_matcher_state_syncs_preview_request_and_highlight() {
        let mut app = App::from_configs(run_config(), search_config(), LoadedAppConfig::default());
        app.search_session.settings.mode = SearchMode::Grep;
        app.set_terminal_size(120, 40);
        app.refresh_viewports();

        let (tx_preview, rx_preview) = unbounded_default();
        apply_matcher_state(
            &mut app,
            crate::search::matcher::MatcherState {
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
