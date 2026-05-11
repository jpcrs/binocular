use crate::app::{App, InputMode};
use crate::preview::PreviewContent;
use arboard::Clipboard;

use super::utils::{get_byte_index, get_line_content};

pub fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

pub fn yank_selection(app: &mut App) {
    if let Some((start_line, start_char)) = app.preview_session.preview.state.selection_start {
        let (end_line, end_char) = (
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );

        let (start, end) =
            if start_line < end_line || (start_line == end_line && start_char <= end_char) {
                ((start_line, start_char), (end_line, end_char))
            } else {
                ((end_line, end_char), (start_line, start_char))
            };

        let mut yanked_text = String::new();

        for i in start.0..=end.0 {
            if let Some(line_content) = get_line_content(app, i) {
                if app.preview_session.preview.state.mode == InputMode::VisualLine {
                    yanked_text.push_str(&line_content);
                } else {
                    let chars: Vec<char> = line_content.chars().collect();
                    let start_idx = if i == start.0 { start.1 } else { 0 };
                    let end_idx = if i == end.0 {
                        end.1.min(chars.len().saturating_sub(1))
                    } else {
                        chars.len().saturating_sub(1)
                    };

                    if start_idx <= end_idx && start_idx < chars.len() {
                        let chunk: String = chars[start_idx..=end_idx].iter().collect();
                        yanked_text.push_str(&chunk);
                    }
                    if i < end.0 {
                        yanked_text.push('\n');
                    }
                }
            }
        }

        copy_to_clipboard(&yanked_text);
    }
}

pub fn yank_line(app: &mut App) {
    if let Some(line_content) = get_line_content(app, app.preview_session.preview.state.cursor_line)
    {
        copy_to_clipboard(&line_content);
    }
}

pub fn yank_range_content(app: &mut App, start: (usize, usize), end: (usize, usize)) {
    if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
        let content = text_file.content();
        let start_idx = get_byte_index(content, start.0, start.1);

        if start.0 > end.0 || (start.0 == end.0 && start.1 > end.1) {
            copy_to_clipboard("");
            return;
        }

        let end_idx = get_byte_index(content, end.0, end.1);
        let end_char_len = content[end_idx..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let yank_end = end_idx + end_char_len;

        if start_idx < yank_end && yank_end <= content.len() {
            let text = &content[start_idx..yank_end];
            copy_to_clipboard(text);
        }
    }
}
