use crate::app::App;
use ratatui::Frame;

pub mod help;
pub mod indicators;
pub mod layout;
pub mod preview;
pub mod search;
pub mod shortcuts;

pub fn draw(f: &mut Frame, app: &mut App) {
    let search_view = search::SearchBarView {
        app_mode: app.ui.mode,
        search_mode: app.search_session.query.mode,
        preview_search_active: app.preview_session.preview.state.search_active,
        preview_search_query: &app.preview_session.preview.state.search_query,
        preview_search_cursor: app
            .preview_session
            .preview
            .state
            .search_query
            .chars()
            .count(),
        query_text: &app.search_session.query.text,
        query_cursor: app.search_session.query.cursor,
        search_label: app
            .search_session
            .settings
            .mode
            .display_name(app.runtime.run.stdin),
        match_mode_label: if app.is_exact_mode() {
            " Exact "
        } else {
            " Fuzzy "
        },
    };
    let results_view = search::SearchResultsView {
        app_mode: app.ui.mode,
        query_mode: app.search_session.query.mode,
        show_preview: app.show_preview(),
        is_content_mode: app.is_content_mode(),
        stdin_mode: app.runtime.run.stdin,
        query_is_empty: app.search_session.query.text.is_empty(),
        total_matches: app.search_session.search.total_matches,
        total_items: app.search_session.search.total_items,
        working: app.search_session.search.working,
        marked_count: app.search_session.search.marked_items.len(),
        diff_marked_count: app.search_session.search.diff_marked_items.len(),
        results: &app.search_session.search.results,
        marked_items: &app.search_session.search.marked_items,
        diff_marked_items: &app.search_session.search.diff_marked_items,
    };
    let preview_view = preview::PreviewView {
        app_mode: app.ui.mode,
        preview_mode: app.preview_session.preview.state.mode,
        source: app.preview_session.preview.source.as_ref(),
        status_message: app
            .preview_session
            .preview
            .state
            .status_message
            .as_ref()
            .map(|(msg, time)| (msg.as_str(), *time)),
        command_buffer: Some(&app.preview_session.preview.state.command_buffer),
        highlight_line: app.preview_session.preview.state.highlight_line,
        search_query: &app.preview_session.preview.state.search_query,
        selection_start: app.preview_session.preview.state.selection_start,
        cursor_line: app.preview_session.preview.state.cursor_line,
        cursor_char: app.preview_session.preview.state.cursor_char,
        scroll: app.preview_session.preview.state.scroll as u16,
        scroll_char: app.preview_session.preview.state.scroll_char as u16,
        area_height: f.area().height,
    };

    // In log mode, the log viewer occupies the entire terminal.
    if app.runtime.run.log {
        preview::render_preview(
            f,
            &preview_view,
            app.preview_session.preview.content.as_mut(),
            f.area(),
        );
        return;
    }

    let areas = layout::split_main_layout(f.area(), app.show_preview(), &app.ui.layout);

    if areas.show_search {
        search::render_search_results(
            f,
            &results_view,
            &mut app.search_session.search.scroll_state,
            areas.search_results,
        );
        search::render_search_bar(f, &search_view, areas.search_bar);
    }

    if let Some(preview_area) = areas.preview {
        let preview_view = preview::PreviewView {
            area_height: preview_area.height,
            ..preview_view
        };
        preview::render_preview(
            f,
            &preview_view,
            app.preview_session.preview.content.as_mut(),
            preview_area,
        );
    }

    help::render_help_modal(f, app);
}
