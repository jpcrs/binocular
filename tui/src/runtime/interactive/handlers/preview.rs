use crate::app::{App, AppAction, InputMode, Mode};
use crate::infra::channel::Sender;
use crate::input::vim;
use crate::output::SelectionOutput;
use crate::preview::{PreviewContent, PreviewRequest, PreviewSource};
use crate::search::types::SearchItem;
use crossterm::event::KeyEvent;
use std::collections::BTreeSet;

pub fn sync_preview(app: &mut App, tx_preview: &impl Sender<PreviewRequest>) {
    if !app.show_preview() {
        app.preview_session.preview.source = None;
        app.preview_session.preview.content = None;
        app.preview_session.preview.state.highlight_line = None;
        return;
    }

    if let Some((left, right)) = marked_diff_paths(app) {
        let preview_source = PreviewSource::Diff {
            left: left.clone(),
            right: right.clone(),
        };
        if Some(&preview_source) != app.preview_session.preview.source.as_ref() {
            app.preview_session.preview.source = Some(preview_source.clone());
            app.preview_session.preview.content = None;
            let _ = tx_preview.send(PreviewRequest::Diff {
                source: preview_source,
                left,
                right,
            });
        }
        app.preview_session.preview.state.scroll = 0;
        app.preview_session.preview.state.highlight_line = None;
        return;
    }

    let item = match app
        .search_session
        .search
        .results
        .get(app.search_session.search.selection)
    {
        Some(result) => &result.item,
        None => return,
    };

    let preview_source = match item {
        SearchItem::GitHistory {
            commit, path, line, ..
        } => PreviewSource::GitHistory {
            commit: commit.clone(),
            path: path.clone(),
            line: *line,
        },
        SearchItem::GitBranch { branch, .. } => PreviewSource::GitBranch {
            branch: branch.clone(),
        },
        SearchItem::GitCommit { commit, .. } => PreviewSource::GitCommit {
            commit: commit.clone(),
        },
        _ => PreviewSource::SearchItem(item.clone()),
    };
    let line_num = item.grep_line().unwrap_or(0);

    if Some(&preview_source) != app.preview_session.preview.source.as_ref() {
        app.preview_session.preview.source = Some(preview_source.clone());
        app.preview_session.preview.content = None;
        let mut preview_request =
            PreviewRequest::from_source(preview_source, app.runtime.run.has_preview_command());
        if let PreviewRequest::GitHistory { repo_root, .. }
        | PreviewRequest::GitBranch { repo_root, .. }
        | PreviewRequest::GitCommit { repo_root, .. } = &mut preview_request
        {
            if let Some(scope) = app.runtime.search.git_search_scope.as_ref() {
                *repo_root = scope.repo_root.display().to_string();
            }
        }
        let _ = tx_preview.send(preview_request);

        if app.is_content_mode() && line_num > 0 {
            center_preview_on_line(app, line_num);
            app.preview_session.preview.state.highlight_line = Some(line_num);
        } else {
            app.preview_session.preview.state.scroll = 0;
            app.preview_session.preview.state.highlight_line = None;
        }
    } else if app.is_content_mode() && line_num > 0 {
        app.preview_session.preview.state.highlight_line = Some(line_num);
        if app.ui.mode == Mode::Search {
            center_preview_on_line(app, line_num);
        }
    } else {
        app.preview_session.preview.state.highlight_line = None;
    }
}

pub fn handle_preview_mode_input(app: &mut App, key: KeyEvent) {
    if matches!(
        app.preview_session.preview.content.as_ref(),
        Some(PreviewContent::StructuredLog(_))
    ) {
        crate::preview::structured_log::handle_input(app, key);
        return;
    }

    if crate::config::kb_matches(&app.keybindings().select_from_preview, &key)
        && !app.preview_session.preview.state.search_active
        && app.preview_session.preview.state.mode != InputMode::Insert
        && app.preview_session.preview.state.mode != InputMode::Command
    {
        if let Some(path) = app
            .preview_session
            .preview
            .source
            .as_ref()
            .and_then(PreviewSource::file_path)
        {
            let abs_path = std::path::PathBuf::from(path)
                .canonicalize()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.to_string());
            let row = app.preview_session.preview.state.cursor_line + 1;
            let col = app.preview_session.preview.state.cursor_char + 1;
            app.set_selected_output(vec![SelectionOutput::PreviewLocation {
                path: abs_path,
                row,
                column: col,
            }]);
            app.apply_action(AppAction::Quit);
        }
    } else {
        vim::handle_input(key, app);
    }
}

pub fn toggle_window_mode(app: &mut App) {
    if !app.show_preview() {
        app.apply_action(AppAction::FocusSearch);
        return;
    }

    if app.ui.layout.preview_fullscreen {
        app.ui.layout.preview_fullscreen = false;
        app.apply_action(AppAction::FocusSearch);
        app.preview_session.preview.state.search_active = false;
        return;
    }

    if app.ui.mode == Mode::Preview {
        app.apply_action(AppAction::FocusSearch);
        app.preview_session.preview.state.search_active = false;
        return;
    }

    if !can_focus_preview(app) {
        app.preview_session.preview.state.status_message = Some((
            "Preview is read-only for this file type".to_string(),
            std::time::Instant::now(),
        ));
        app.apply_action(AppAction::FocusSearch);
        return;
    }

    app.apply_action(AppAction::FocusPreview);
    if let Some(line) = app.preview_session.preview.state.highlight_line {
        app.preview_session.preview.state.cursor_line = line.saturating_sub(1);
    } else {
        app.preview_session.preview.state.cursor_line = app.preview_session.preview.state.scroll;
    }
    app.preview_session.preview.state.cursor_char = 0;
}

pub fn scroll_preview_page(app: &mut App, down: bool) {
    const SCROLL_LINES: usize = 21;
    let total = preview_line_count(app);
    let page = app.preview_height().saturating_sub(2) as usize;
    if down {
        let max_scroll = total.saturating_sub(page);
        app.preview_session.preview.state.scroll =
            (app.preview_session.preview.state.scroll + SCROLL_LINES).min(max_scroll);
    } else {
        app.preview_session.preview.state.scroll = app
            .preview_session
            .preview
            .state
            .scroll
            .saturating_sub(SCROLL_LINES);
    }
}

fn center_preview_on_line(app: &mut App, line: usize) {
    let height = if app.preview_height() > 2 {
        app.preview_height() - 2
    } else {
        20
    };
    app.preview_session.preview.state.scroll = line.saturating_sub(height as usize / 2);
}

fn can_focus_preview(app: &App) -> bool {
    if matches!(
        app.preview_session.preview.source.as_ref(),
        Some(PreviewSource::GitHistory { .. })
    ) {
        return false;
    }

    match app.preview_session.preview.content.as_ref() {
        Some(PreviewContent::RichText(_)) | Some(PreviewContent::StructuredLog(_)) => true,
        Some(PreviewContent::Diff(_))
        | Some(PreviewContent::PlainText(_))
        | Some(PreviewContent::Image(_))
        | Some(PreviewContent::Media(_)) => false,
        None => true,
    }
}

fn preview_line_count(app: &App) -> usize {
    match app.preview_session.preview.content.as_ref() {
        Some(PreviewContent::RichText(tf)) => tf.raw_lines().len(),
        Some(PreviewContent::Diff(diff)) => diff.text.lines.len(),
        Some(PreviewContent::PlainText(text)) => text.lines.len(),
        Some(PreviewContent::Media(m)) => m.metadata.lines.len(),
        Some(PreviewContent::StructuredLog(lp)) => lp.filter_state.cached_matches.len(),
        _ => 0,
    }
}

fn marked_diff_paths(app: &App) -> Option<(String, String)> {
    let paths = app
        .search_session
        .search
        .diff_marked_items
        .iter()
        .filter_map(|item| match item {
            SearchItem::Path(path) => Some(path.clone()),
            SearchItem::Grep { path, .. } => Some(path.clone()),
            SearchItem::GitHistory { .. }
            | SearchItem::GitBranch { .. }
            | SearchItem::GitCommit { .. }
            | SearchItem::Stdin(_)
            | SearchItem::Message(_) => None,
        })
        .collect::<BTreeSet<_>>();

    if paths.len() != 2 {
        return None;
    }

    let mut iter = paths.into_iter();
    Some((iter.next()?, iter.next()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoadedAppConfig;
    use crate::preview::DiffPreview;
    use crate::runtime::config::RunConfig;
    use crate::search::types::{MatcherMode, SearchConfig, SearchMode, SearchSettings};
    use ratatui::text::Text;

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

    #[test]
    fn diff_preview_scroll_uses_diff_line_count() {
        let mut app = app();
        app.set_terminal_size(120, 10);
        app.refresh_viewports();
        app.preview_session.preview.content = Some(PreviewContent::Diff(DiffPreview {
            text: Text::from(
                (0..40)
                    .map(|idx| format!("line {idx}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        }));

        scroll_preview_page(&mut app, true);

        assert!(app.preview_session.preview.state.scroll > 0);
    }
}
