use crate::app::App;
use crate::preview::rich_text::TextUndoFrame;
use crate::preview::PreviewContent;

use super::undo::record_edit;
use super::utils::{ensure_cursor_in_bounds, get_byte_index, get_line_count, line_len};

fn push_edit_frame(
    app: &mut App,
    edit: crate::preview::rich_text::TextEdit,
    before_cursor: (usize, usize),
    after_cursor: (usize, usize),
) {
    record_edit(
        app,
        TextUndoFrame::from_forward_edit(edit, before_cursor, after_cursor),
    );
}

pub fn insert_char(app: &mut App, c: char) {
    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );
    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let cursor_idx = text_file
            .buffer
            .byte_index(before_cursor.0, before_cursor.1);

        if cursor_idx <= text_file.len_bytes() {
            if let Some(edit) = crate::preview::edit_content_insert_char(text_file, cursor_idx, c) {
                app.preview_session.preview.state.cursor_char += 1;
                push_edit_frame(
                    app,
                    edit,
                    before_cursor,
                    (
                        app.preview_session.preview.state.cursor_line,
                        app.preview_session.preview.state.cursor_char,
                    ),
                );
            }
        }
    }
}

pub fn insert_newline(app: &mut App) {
    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );
    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let cursor_idx = text_file
            .buffer
            .byte_index(before_cursor.0, before_cursor.1);

        if cursor_idx <= text_file.len_bytes() {
            if let Some(edit) =
                crate::preview::edit_content_insert_char(text_file, cursor_idx, '\n')
            {
                push_edit_frame(app, edit, before_cursor, (before_cursor.0 + 1, 0));
            }
            app.preview_session.preview.state.cursor_line += 1;
            app.preview_session.preview.state.cursor_char = 0;
        }
    }
}

pub fn delete_char(app: &mut App) {
    let cursor_idx =
        if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
            let idx = get_byte_index(
                text_file.content(),
                app.preview_session.preview.state.cursor_line,
                app.preview_session.preview.state.cursor_char,
            );
            if idx > 0 {
                Some(idx)
            } else {
                None
            }
        } else {
            None
        };

    if let Some(cursor_idx) = cursor_idx {
        let before_cursor = (
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );

        let deleting_newline = app.preview_session.preview.state.cursor_char == 0
            && app.preview_session.preview.state.cursor_line > 0;

        let new_cursor_char = if deleting_newline {
            if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content
            {
                if let Some((start, end)) =
                    text_file.line_range(app.preview_session.preview.state.cursor_line - 1)
                {
                    let prev_line = &text_file.content()[start..end];
                    prev_line
                        .trim_end_matches('\n')
                        .trim_end_matches('\r')
                        .chars()
                        .count()
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            app.preview_session.preview.state.cursor_char - 1
        };

        if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content
        {
            if let Some(edit) = crate::preview::edit_content_delete_char(text_file, cursor_idx) {
                if deleting_newline {
                    app.preview_session.preview.state.cursor_line -= 1;
                    app.preview_session.preview.state.cursor_char = new_cursor_char;
                } else {
                    app.preview_session.preview.state.cursor_char = new_cursor_char;
                }
                push_edit_frame(
                    app,
                    edit,
                    before_cursor,
                    (
                        app.preview_session.preview.state.cursor_line,
                        app.preview_session.preview.state.cursor_char,
                    ),
                );
            }
        }
    }
}

pub fn delete_char_under_cursor(app: &mut App) {
    let cursor_idx =
        if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
            let idx = get_byte_index(
                text_file.content(),
                app.preview_session.preview.state.cursor_line,
                app.preview_session.preview.state.cursor_char,
            );
            if idx < text_file.len_bytes() {
                let c = text_file.content()[idx..].chars().next();
                if c.map(|c| c != '\n').unwrap_or(false) {
                    Some(idx)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

    if let Some(cursor_idx) = cursor_idx {
        let before_cursor = (
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );
        if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content
        {
            if let Some(edit) = crate::preview::edit_content_delete_char_at(text_file, cursor_idx) {
                let len = text_file
                    .line_slice(app.preview_session.preview.state.cursor_line)
                    .map(|line| {
                        line.trim_end_matches('\n')
                            .trim_end_matches('\r')
                            .chars()
                            .count()
                    })
                    .unwrap_or(0);
                if app.preview_session.preview.state.cursor_char >= len && len > 0 {
                    app.preview_session.preview.state.cursor_char = len - 1;
                } else if len == 0 {
                    app.preview_session.preview.state.cursor_char = 0;
                }
                push_edit_frame(
                    app,
                    edit,
                    before_cursor,
                    (
                        app.preview_session.preview.state.cursor_line,
                        app.preview_session.preview.state.cursor_char,
                    ),
                );
            }
        }
    }
}

pub fn delete_range(app: &mut App, start: (usize, usize), end: (usize, usize)) {
    let should_delete =
        if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
            let content = text_file.content();
            let start_idx = get_byte_index(content, start.0, start.1);

            if start.0 > end.0 || (start.0 == end.0 && start.1 > end.1) {
                false
            } else {
                let end_idx = get_byte_index(content, end.0, end.1);
                let end_char_len = content[end_idx..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                let delete_end = end_idx + end_char_len;

                start_idx < delete_end && delete_end <= content.len()
            }
        } else {
            false
        };

    if should_delete {
        let before_cursor = (
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );
        if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content
        {
            let content = text_file.content().to_string();
            let start_idx = get_byte_index(&content, start.0, start.1);
            let end_idx = get_byte_index(&content, end.0, end.1);
            let end_char_len = content[end_idx..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            let delete_end = end_idx + end_char_len;

            if let Some(edit) =
                crate::preview::edit_content_delete_range(text_file, start_idx, delete_end)
            {
                app.preview_session.preview.state.cursor_line = start.0;
                app.preview_session.preview.state.cursor_char = start.1;
                ensure_cursor_in_bounds(app);
                push_edit_frame(
                    app,
                    edit,
                    before_cursor,
                    (
                        app.preview_session.preview.state.cursor_line,
                        app.preview_session.preview.state.cursor_char,
                    ),
                );
            }
        }
    } else if start.0 > end.0 || (start.0 == end.0 && start.1 > end.1) {
        app.preview_session.preview.state.cursor_line = start.0;
        app.preview_session.preview.state.cursor_char = start.1;
    }
}

pub fn delete_line(app: &mut App, count: usize) {
    let line_count = get_line_count(app);
    if line_count == 0 {
        return;
    }

    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );

    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let start_line = app.preview_session.preview.state.cursor_line;
        let end_line = (start_line + count).min(line_count);

        let start_byte = if let Some((start, _)) = text_file.line_range(start_line) {
            start
        } else {
            return;
        };

        let end_byte = if end_line >= line_count {
            text_file.len_bytes()
        } else if let Some((start, _)) = text_file.line_range(end_line) {
            start
        } else {
            text_file.len_bytes()
        };

        if let Some(edit) =
            crate::preview::edit_content_delete_range(text_file, start_byte, end_byte)
        {
            let new_line_count = text_file.line_count();
            if new_line_count == 0 {
                app.preview_session.preview.state.cursor_line = 0;
                app.preview_session.preview.state.cursor_char = 0;
            } else if app.preview_session.preview.state.cursor_line >= new_line_count {
                app.preview_session.preview.state.cursor_line = new_line_count - 1;
                app.preview_session.preview.state.cursor_char = 0;
            } else {
                app.preview_session.preview.state.cursor_char = 0;
            }

            let deleted = end_line - start_line;
            app.preview_session.preview.state.status_message = Some((
                format!("{} line(s) deleted", deleted),
                std::time::Instant::now(),
            ));
            push_edit_frame(
                app,
                edit,
                before_cursor,
                (
                    app.preview_session.preview.state.cursor_line,
                    app.preview_session.preview.state.cursor_char,
                ),
            );
        }
    }

    ensure_cursor_in_bounds(app);
}

pub fn delete_line_content(app: &mut App) {
    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );

    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        if let Some((start, end)) =
            text_file.line_range(app.preview_session.preview.state.cursor_line)
        {
            let line_content = &text_file.content()[start..end];
            let has_newline = line_content.ends_with('\n');

            let delete_end = if has_newline {
                end.saturating_sub(1)
            } else {
                end
            };

            if let Some(edit) =
                crate::preview::edit_content_delete_range(text_file, start, delete_end)
            {
                push_edit_frame(
                    app,
                    edit,
                    before_cursor,
                    (
                        app.preview_session.preview.state.cursor_line,
                        app.preview_session.preview.state.cursor_char,
                    ),
                );
            }
        }

        app.preview_session.preview.state.cursor_char = 0;
    }

    ensure_cursor_in_bounds(app);
}

pub fn delete_to_end_of_line(app: &mut App) {
    let content_len = line_len(app, app.preview_session.preview.state.cursor_line);
    if content_len == 0 || app.preview_session.preview.state.cursor_char >= content_len {
        return;
    }

    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );

    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let cursor_byte = get_byte_index(
            text_file.content(),
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );

        if let Some((_, end)) = text_file.line_range(app.preview_session.preview.state.cursor_line)
        {
            let actual_end =
                if end > 0 && text_file.content().as_bytes().get(end - 1) == Some(&b'\n') {
                    end - 1
                } else {
                    end
                };

            if cursor_byte < actual_end {
                if let Some(edit) =
                    crate::preview::edit_content_delete_range(text_file, cursor_byte, actual_end)
                {
                    push_edit_frame(
                        app,
                        edit,
                        before_cursor,
                        (
                            app.preview_session.preview.state.cursor_line,
                            app.preview_session.preview.state.cursor_char,
                        ),
                    );
                }
            }
        }
    }

    ensure_cursor_in_bounds(app);
}

pub fn open_line_below(app: &mut App) {
    let cursor_line = app.preview_session.preview.state.cursor_line;
    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );

    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let insert_pos = if let Some((_, end)) = text_file.line_range(cursor_line) {
            end
        } else {
            text_file.len_bytes()
        };

        if let Some(edit) = crate::preview::edit_content_insert_char(text_file, insert_pos, '\n') {
            app.preview_session.preview.state.cursor_line += 1;
            app.preview_session.preview.state.cursor_char = 0;
            push_edit_frame(
                app,
                edit,
                before_cursor,
                (
                    app.preview_session.preview.state.cursor_line,
                    app.preview_session.preview.state.cursor_char,
                ),
            );
        }
    }

    ensure_cursor_in_bounds(app);
}

pub fn open_line_above(app: &mut App) {
    let before_cursor = (
        app.preview_session.preview.state.cursor_line,
        app.preview_session.preview.state.cursor_char,
    );

    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        let insert_pos = if let Some((start, _)) =
            text_file.line_range(app.preview_session.preview.state.cursor_line)
        {
            start
        } else {
            0
        };

        if let Some(edit) = crate::preview::edit_content_insert_char(text_file, insert_pos, '\n') {
            app.preview_session.preview.state.cursor_char = 0;
            push_edit_frame(
                app,
                edit,
                before_cursor,
                (
                    app.preview_session.preview.state.cursor_line,
                    app.preview_session.preview.state.cursor_char,
                ),
            );
        }
    }

    ensure_cursor_in_bounds(app);
}
