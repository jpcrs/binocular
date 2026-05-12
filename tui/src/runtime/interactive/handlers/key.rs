use super::preview::{scroll_preview_page, sync_preview, toggle_window_mode};
use super::{handle_help_modal_input, handle_preview_mode_input, handle_search_mode_input};
use crate::app::{App, AppAction, Mode};
use crate::config::kb_matches;
use crate::infra::channel::Sender;
use crate::preview::PreviewRequest;
use crate::search::matcher::MatcherCommand;
use crate::search::types::SearchMode;
use crossterm::event::KeyEvent;

pub fn handle_input(
    app: &mut App,
    key: KeyEvent,
    tx_cmd: &impl Sender<MatcherCommand>,
    tx_preview: &impl Sender<PreviewRequest>,
) {
    if kb_matches(&app.keybindings().quit, &key) {
        app.apply_action(AppAction::Quit);
        return;
    }

    if kb_matches(&app.keybindings().toggle_help, &key) {
        app.apply_action(AppAction::ToggleHelp);
        return;
    }

    if app.ui.help.visible {
        handle_help_modal_input(app, key);
        return;
    }

    if kb_matches(&app.keybindings().toggle_preview_focus, &key) {
        if app.show_preview() {
            toggle_window_mode(app);
        }
        return;
    }

    if app.show_preview() {
        if kb_matches(&app.keybindings().toggle_preview_fullscreen, &key) {
            app.apply_action(AppAction::TogglePreviewFullscreen);
            return;
        }
        if kb_matches(&app.keybindings().swap_panes, &key) {
            app.apply_action(AppAction::SwapPanes);
            return;
        }
        if kb_matches(&app.keybindings().preview_wider, &key) {
            app.apply_action(AppAction::AdjustPreviewWidth(5));
            return;
        }
        if kb_matches(&app.keybindings().preview_narrower, &key) {
            app.apply_action(AppAction::AdjustPreviewWidth(-5));
            return;
        }
    }
    if kb_matches(&app.keybindings().toggle_search_bar_position, &key) {
        app.apply_action(AppAction::ToggleSearchBarPosition);
        return;
    }
    if kb_matches(&app.keybindings().toggle_preview_visibility, &key) {
        let naturally_visible = !app.runtime.run.stdin || app.runtime.run.has_preview_command();
        if naturally_visible {
            app.apply_action(AppAction::TogglePreviewVisibility);
            if !app.ui.layout.preview_hidden {
                sync_preview(app, tx_preview);
            }
        }
        return;
    }

    if kb_matches(&app.keybindings().toggle_exact, &key) {
        app.apply_action(AppAction::ToggleExactMatcher);
        return;
    }

    if !app.runtime.run.stdin {
        if kb_matches(&app.keybindings().mode_path, &key) {
            app.apply_action(AppAction::SetSearchMode(SearchMode::Path));
            return;
        }
        if kb_matches(&app.keybindings().mode_files, &key) {
            app.apply_action(AppAction::SetSearchMode(SearchMode::Files));
            return;
        }
        if kb_matches(&app.keybindings().mode_grep, &key) {
            app.apply_action(AppAction::SetSearchMode(SearchMode::Grep));
            return;
        }
        if kb_matches(&app.keybindings().mode_dirs, &key) {
            app.apply_action(AppAction::SetSearchMode(SearchMode::Dirs));
            return;
        }
    }

    if app.show_preview() && app.ui.mode != Mode::Preview {
        if kb_matches(&app.keybindings().scroll_preview_up, &key) {
            scroll_preview_page(app, false);
            return;
        }
        if kb_matches(&app.keybindings().scroll_preview_down, &key) {
            scroll_preview_page(app, true);
            return;
        }
    }

    if app.ui.mode == Mode::Preview {
        handle_preview_mode_input(app, key);
    } else {
        handle_search_mode_input(app, key, tx_cmd, tx_preview);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoadedAppConfig;
    use crate::infra::channel::unbounded_default;
    use crate::preview::{create_rich_text_document, PreviewContent, PreviewSource};
    use crate::runtime::config::RunConfig;
    use crate::search::types::{MatcherMode, SearchConfig, SearchItem, SearchMode, SearchSettings};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::Path;

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

    fn ctrl(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
    }

    #[test]
    fn mode_shortcuts_switch_search_mode_and_request_restart() {
        let mut app = app();
        let (tx_cmd, _rx_cmd) = unbounded_default();
        let (tx_preview, _rx_preview) = unbounded_default();

        handle_input(&mut app, key(KeyCode::F(3)), &tx_cmd, &tx_preview);
        assert_eq!(app.search_session.settings.mode, SearchMode::Grep);
        assert!(app.ui.restart_search);

        app.ui.restart_search = false;
        handle_input(&mut app, key(KeyCode::F(4)), &tx_cmd, &tx_preview);
        assert_eq!(app.search_session.settings.mode, SearchMode::Dirs);
        assert!(app.ui.restart_search);
    }

    #[test]
    fn preview_focus_stays_on_search_for_read_only_preview() {
        let mut app = app();
        app.preview_session.preview.content = Some(PreviewContent::PlainText("binary".into()));

        let (tx_cmd, _rx_cmd) = unbounded_default();
        let (tx_preview, _rx_preview) = unbounded_default();
        handle_input(&mut app, ctrl('w'), &tx_cmd, &tx_preview);

        assert_eq!(app.ui.mode, Mode::Search);
        assert!(app.preview_session.preview.state.status_message.is_some());
    }

    #[test]
    fn preview_focus_moves_to_text_preview() {
        let mut app = app();
        app.preview_session.preview.state.highlight_line = Some(3);
        app.preview_session.preview.content =
            Some(PreviewContent::RichText(create_rich_text_document(
                "first\nsecond\nthird\nfourth\n".to_string(),
                Path::new("test.txt"),
            )));
        app.preview_session.preview.source =
            Some(PreviewSource::SearchItem(SearchItem::path("test.txt")));

        let (tx_cmd, _rx_cmd) = unbounded_default();
        let (tx_preview, _rx_preview) = unbounded_default();
        handle_input(&mut app, ctrl('w'), &tx_cmd, &tx_preview);

        assert_eq!(app.ui.mode, Mode::Preview);
        assert_eq!(app.preview_session.preview.state.cursor_line, 2);
        assert_eq!(app.preview_session.preview.state.cursor_char, 0);
    }
}
