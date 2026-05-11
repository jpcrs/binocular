use crate::app::App;
use crate::preview::rich_text::TextUndoFrame;
use crate::preview::PreviewContent;

use super::utils::ensure_cursor_in_bounds;

pub fn record_edit(app: &mut App, frame: TextUndoFrame) {
    app.preview_session.preview.state.undo_stack.push(frame);
    app.preview_session.preview.state.redo_stack.clear();
}

pub fn perform_undo(app: &mut App) {
    if let Some(frame) = app.preview_session.preview.state.undo_stack.pop() {
        if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content
        {
            if crate::preview::apply_text_edit(text_file, &frame.undo_edit) {
                app.preview_session
                    .preview
                    .state
                    .redo_stack
                    .push(frame.clone());
                app.preview_session.preview.state.cursor_line = frame.before_cursor.0;
                app.preview_session.preview.state.cursor_char = frame.before_cursor.1;
                ensure_cursor_in_bounds(app);
                app.preview_session.preview.state.status_message =
                    Some(("Undo".to_string(), std::time::Instant::now()));
                return;
            }
        }
        app.preview_session.preview.state.status_message =
            Some(("Undo failed".to_string(), std::time::Instant::now()));
    } else {
        app.preview_session.preview.state.status_message = Some((
            "Already at oldest change".to_string(),
            std::time::Instant::now(),
        ));
    }
}

pub fn perform_redo(app: &mut App) {
    if let Some(frame) = app.preview_session.preview.state.redo_stack.pop() {
        if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content
        {
            if crate::preview::apply_text_edit(text_file, &frame.redo_edit) {
                app.preview_session
                    .preview
                    .state
                    .undo_stack
                    .push(frame.clone());
                app.preview_session.preview.state.cursor_line = frame.after_cursor.0;
                app.preview_session.preview.state.cursor_char = frame.after_cursor.1;
                ensure_cursor_in_bounds(app);
                app.preview_session.preview.state.status_message =
                    Some(("Redo".to_string(), std::time::Instant::now()));
                return;
            }
        }
        app.preview_session.preview.state.status_message =
            Some(("Redo failed".to_string(), std::time::Instant::now()));
    } else {
        app.preview_session.preview.state.status_message = Some((
            "Already at newest change".to_string(),
            std::time::Instant::now(),
        ));
    }
}
