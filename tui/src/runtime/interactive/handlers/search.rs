use super::preview::sync_preview;
use crate::app::{App, AppAction};
use crate::config::kb_matches;
use crate::infra::channel::Sender;
use crate::input::vim;
use crate::output::SelectionOutput;
use crate::preview::PreviewRequest;
use crate::search::matcher::MatcherCommand;
use crate::search::types::SearchItem;
use crossterm::event::KeyEvent;
use std::collections::BTreeSet;

pub fn handle_search_mode_input(
    app: &mut App,
    key: KeyEvent,
    tx_cmd: &impl Sender<MatcherCommand>,
    tx_preview: &impl Sender<PreviewRequest>,
) {
    if kb_matches(&app.keybindings().mark_result, &key) {
        if let Some(result) = app
            .search_session
            .search
            .results
            .get(app.search_session.search.selection)
        {
            let item_key = result.item.clone();
            match app.search_session.search.marked_items.entry(item_key) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    e.remove();
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(result.column);
                }
            }
            app.search_session.search.next();
            sync_preview(app, tx_preview);
        }
    } else if kb_matches(&app.keybindings().mark_diff_result, &key) {
        if let Some(result) = app
            .search_session
            .search
            .results
            .get(app.search_session.search.selection)
        {
            let item_key = result.item.clone();
            let already_marked = app
                .search_session
                .search
                .diff_marked_items
                .contains(&item_key);
            if !already_marked {
                if let Some(message) = validate_diff_mark(app, &item_key) {
                    app.preview_session.preview.state.status_message =
                        Some((message, std::time::Instant::now()));
                    return;
                }
            }

            if already_marked {
                app.search_session
                    .search
                    .diff_marked_items
                    .remove(&item_key);
            } else {
                app.search_session.search.diff_marked_items.insert(item_key);
            }

            app.search_session.search.next();
            sync_preview(app, tx_preview);
        }
    } else {
        let old_query = app.search_session.query.text.clone();
        let result = vim::handle_search_input(key, app);

        match result {
            vim::SearchInputResult::QueryChanged => {
                if app.search_session.query.text != old_query {
                    app.search_session.search.selection = 0;
                    app.search_session.search.selected_item = None;
                    app.search_session.search.scroll_state.select(Some(0));
                    let _ =
                        tx_cmd.send(MatcherCommand::Query(app.search_session.query.text.clone()));
                }
            }
            vim::SearchInputResult::ListUp(count) => {
                for _ in 0..count {
                    app.search_session.search.previous();
                }
                sync_preview(app, tx_preview);
            }
            vim::SearchInputResult::ListDown(count) => {
                for _ in 0..count {
                    app.search_session.search.next();
                }
                sync_preview(app, tx_preview);
            }
            vim::SearchInputResult::Select => {
                select_current_item(app);
            }
            vim::SearchInputResult::Quit => {
                app.apply_action(AppAction::Quit);
            }
            vim::SearchInputResult::None => {}
        }
    }
}

pub(crate) fn validate_diff_mark(app: &App, item: &SearchItem) -> Option<String> {
    let Some(path) = item.preview_path() else {
        return Some("Diff mode only supports file-backed results".to_string());
    };

    let existing_paths = app
        .search_session
        .search
        .diff_marked_items
        .iter()
        .filter_map(SearchItem::preview_path)
        .collect::<BTreeSet<_>>();

    if existing_paths.contains(path) {
        return Some("Diff mode requires two distinct files".to_string());
    }

    if existing_paths.len() >= 2 {
        return Some("Diff mode allows marking at most two files".to_string());
    }

    None
}

fn select_current_item(app: &mut App) {
    let items_to_output: Vec<SelectionOutput> = if app.search_session.search.marked_items.is_empty()
    {
        if let Some(result) = app
            .search_session
            .search
            .results
            .get(app.search_session.search.selection)
        {
            vec![SelectionOutput::Item {
                item: result.item.clone(),
                column: result.column,
            }]
        } else {
            vec![]
        }
    } else {
        let mut marked: Vec<_> = app.search_session.search.marked_items.iter().collect();
        marked.sort_by(|a, b| a.0.display_text().cmp(&b.0.display_text()));
        marked
            .iter()
            .map(|(item, column)| SelectionOutput::Item {
                item: (*item).clone(),
                column: **column,
            })
            .collect()
    };

    if !items_to_output.is_empty() {
        app.set_selected_output(items_to_output);
    }
    app.apply_action(AppAction::Quit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoadedAppConfig;
    use crate::infra::channel::{unbounded_default, Receiver};
    use crate::runtime::config::RunConfig;
    use crate::search::types::{
        MatcherMode, SearchConfig, SearchMode, SearchResult, SearchSettings,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn app() -> App {
        App::from_configs(
            RunConfig {
                headless: false,
                output_format: crate::cli::args::OutputFormat::Plain,
                output_file: None,
                stdin: false,
                log: false,
                diff: None,
                preview_command: None,
                preview_delimiter: ":".to_string(),
                split: None,
                log_files: Vec::new(),
            },
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
            },
            LoadedAppConfig::default(),
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn diff_marks_two_files_and_requests_diff_preview() {
        let mut app = app();
        app.search_session.search.results = vec![
            SearchResult {
                item: SearchItem::path("b.txt"),
                indices: vec![],
                column: None,
            },
            SearchResult {
                item: SearchItem::path("a.txt"),
                indices: vec![],
                column: None,
            },
        ];
        app.search_session.search.total_matches = 2;
        app.search_session.search.total_items = 2;

        let (tx_cmd, _rx_cmd) = unbounded_default();
        let (tx_preview, rx_preview) = unbounded_default();

        handle_search_mode_input(&mut app, key(KeyCode::F(5)), &tx_cmd, &tx_preview);
        let first_request = rx_preview.recv().expect("first preview request");
        assert!(matches!(first_request, PreviewRequest::Path { .. }));

        handle_search_mode_input(&mut app, key(KeyCode::F(5)), &tx_cmd, &tx_preview);
        let second_request = rx_preview.recv().expect("diff preview request");
        assert_eq!(
            second_request,
            PreviewRequest::Diff {
                source: crate::preview::PreviewSource::Diff {
                    left: "a.txt".to_string(),
                    right: "b.txt".to_string(),
                },
                left: "a.txt".to_string(),
                right: "b.txt".to_string(),
            }
        );
    }

    #[test]
    fn diff_mark_rejects_third_mark() {
        let mut app = app();
        app.search_session
            .search
            .diff_marked_items
            .insert(SearchItem::path("a.txt"));
        app.search_session
            .search
            .diff_marked_items
            .insert(SearchItem::path("b.txt"));

        let message = validate_diff_mark(&app, &SearchItem::path("c.txt"));
        assert_eq!(
            message,
            Some("Diff mode allows marking at most two files".to_string())
        );
    }

    #[test]
    fn diff_mark_allows_unmarking_existing_file() {
        let mut app = app();
        app.search_session.search.results = vec![SearchResult {
            item: SearchItem::path("a.txt"),
            indices: vec![],
            column: None,
        }];
        app.search_session.search.total_matches = 1;
        app.search_session.search.total_items = 1;
        app.search_session
            .search
            .diff_marked_items
            .insert(SearchItem::path("a.txt"));

        let (tx_cmd, _rx_cmd) = unbounded_default();
        let (tx_preview, _rx_preview) = unbounded_default();
        handle_search_mode_input(&mut app, key(KeyCode::F(5)), &tx_cmd, &tx_preview);

        assert!(app.search_session.search.diff_marked_items.is_empty());
    }
}
