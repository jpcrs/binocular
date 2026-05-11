use crate::app::App;
use crate::preview::PreviewContent;

use super::utils::{get_byte_index, get_line_char_from_byte, get_line_content};

fn adjust_inner_range(
    app: &App,
    start: (usize, usize),
    end: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
    let mut inner_start = start;
    let mut inner_end = end;

    if let Some(line_content) = get_line_content(app, inner_start.0) {
        let chars: Vec<char> = line_content.chars().collect();
        if inner_start.1 + 1 < chars.len() {
            inner_start.1 += 1;
        } else {
            inner_start.0 += 1;
            inner_start.1 = 0;
        }
    }

    if inner_end.1 > 0 {
        inner_end.1 -= 1;
    } else if inner_end.0 > 0 {
        inner_end.0 -= 1;
        if let Some(line_content) = get_line_content(app, inner_end.0) {
            inner_end.1 = line_content.chars().count().saturating_sub(1);
        }
    }

    if inner_start.0 < inner_end.0 || (inner_start.0 == inner_end.0 && inner_start.1 <= inner_end.1)
    {
        (inner_start, inner_end)
    } else {
        (inner_start, inner_start)
    }
}

fn word_bounds_at_cursor(app: &App) -> Option<(usize, usize)> {
    let line_content = get_line_content(app, app.preview_session.preview.state.cursor_line)?;
    let chars: Vec<char> = line_content.chars().collect();
    let cursor_char = app.preview_session.preview.state.cursor_char;

    if cursor_char >= chars.len() {
        return None;
    }

    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

    if !is_word_char(chars[cursor_char]) {
        return None;
    }

    let mut start = cursor_char;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = cursor_char;
    while end + 1 < chars.len() && is_word_char(chars[end + 1]) {
        end += 1;
    }

    Some((start, end))
}

pub fn find_inner_quote_range(app: &App, quote: char) -> Option<((usize, usize), (usize, usize))> {
    let mut start_pos = None;

    let mut line_idx = app.preview_session.preview.state.cursor_line;
    let mut char_idx = app.preview_session.preview.state.cursor_char;

    loop {
        if let Some(line_content) = get_line_content(app, line_idx) {
            let chars: Vec<char> = line_content.chars().collect();

            if char_idx == usize::MAX {
                char_idx = chars.len().saturating_sub(1);
                if chars.is_empty() {
                    if line_idx > 0 {
                        line_idx -= 1;
                        char_idx = usize::MAX;
                        continue;
                    } else {
                        break;
                    }
                }
            } else if char_idx >= chars.len() {
                if line_idx > 0 {
                    line_idx -= 1;
                    char_idx = usize::MAX;
                    continue;
                } else {
                    break;
                }
            }

            let c = chars[char_idx];
            if c == quote {
                let is_escaped = char_idx > 0 && chars[char_idx - 1] == '\\';
                if !is_escaped {
                    start_pos = Some((line_idx, char_idx));
                    break;
                }
            }

            if char_idx > 0 {
                char_idx -= 1;
            } else if line_idx > 0 {
                line_idx -= 1;
                char_idx = usize::MAX;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let start = start_pos?;
    let mut end_pos = None;

    line_idx = start.0;
    char_idx = start.1 + 1;

    loop {
        if let Some(line_content) = get_line_content(app, line_idx) {
            let chars: Vec<char> = line_content.chars().collect();

            if char_idx >= chars.len() {
                line_idx += 1;
                char_idx = 0;
                continue;
            }

            let c = chars[char_idx];
            if c == quote {
                let is_escaped = char_idx > 0 && chars[char_idx - 1] == '\\';
                if !is_escaped {
                    end_pos = Some((line_idx, char_idx));
                    break;
                }
            }

            char_idx += 1;
        } else {
            break;
        }
    }

    end_pos.map(|end| adjust_inner_range(app, start, end))
}

pub fn find_inner_pair_range(
    app: &App,
    open: char,
    close: char,
) -> Option<((usize, usize), (usize, usize))> {
    if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
        if let Some(tree) = &text_file.tree {
            let cursor_byte = get_byte_index(
                text_file.content(),
                app.preview_session.preview.state.cursor_line,
                app.preview_session.preview.state.cursor_char,
            );
            let root = tree.root_node();

            let mut node = root
                .descendant_for_byte_range(cursor_byte, cursor_byte)
                .unwrap_or(root);

            loop {
                let start_byte = node.start_byte();
                let end_byte = node.end_byte();

                if end_byte > start_byte {
                    let node_text = &text_file.content()[start_byte..end_byte];
                    if node_text.trim().starts_with(open) && node_text.trim().ends_with(close) {
                        let open_offset = node_text.find(open).unwrap_or(0);
                        let close_offset = node_text.rfind(close).unwrap_or(node_text.len());

                        let inner_start_byte = start_byte + open_offset + open.len_utf8();
                        let inner_end_byte = start_byte + close_offset;

                        if inner_start_byte > inner_end_byte {
                            let (start_line, start_char) =
                                get_line_char_from_byte(text_file.content(), inner_start_byte);
                            return Some(((start_line, start_char), (start_line, start_char)));
                        }

                        let (start_line, start_char) =
                            get_line_char_from_byte(text_file.content(), inner_start_byte);

                        let prev_char_byte = text_file.content()[..inner_end_byte]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(inner_start_byte);
                        let (end_line, end_char) =
                            get_line_char_from_byte(text_file.content(), prev_char_byte);

                        return Some(((start_line, start_char), (end_line, end_char)));
                    }
                }

                if let Some(parent) = node.parent() {
                    node = parent;
                } else {
                    break;
                }
            }
        }
    }

    find_inner_pair_range_fallback(app, open, close)
}

fn find_inner_pair_range_fallback(
    app: &App,
    open: char,
    close: char,
) -> Option<((usize, usize), (usize, usize))> {
    let mut start_pos = None;
    let mut depth = 0;

    let mut line_idx = app.preview_session.preview.state.cursor_line;
    let mut char_idx = app.preview_session.preview.state.cursor_char;

    loop {
        if let Some(line_content) = get_line_content(app, line_idx) {
            let chars: Vec<char> = line_content.chars().collect();
            if char_idx >= chars.len() && char_idx != usize::MAX {
                if line_idx > 0 {
                    line_idx -= 1;
                    char_idx = usize::MAX;
                    continue;
                } else {
                    break;
                }
            }

            if char_idx == usize::MAX {
                char_idx = chars.len().saturating_sub(1);
                if chars.is_empty() {
                    if line_idx > 0 {
                        line_idx -= 1;
                        char_idx = usize::MAX;
                        continue;
                    } else {
                        break;
                    }
                }
            } else if char_idx >= chars.len() {
                if line_idx > 0 {
                    line_idx -= 1;
                    char_idx = usize::MAX;
                    continue;
                } else {
                    break;
                }
            }

            let c = chars[char_idx];
            if c == close {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    start_pos = Some((line_idx, char_idx));
                    break;
                }
                depth -= 1;
            }

            if char_idx > 0 {
                char_idx -= 1;
            } else if line_idx > 0 {
                line_idx -= 1;
                char_idx = usize::MAX;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let start = start_pos?;
    let mut end_pos = None;
    depth = 0;

    line_idx = start.0;

    loop {
        if let Some(line_content) = get_line_content(app, line_idx) {
            let chars: Vec<char> = line_content.chars().collect();

            if char_idx >= chars.len() {
                line_idx += 1;
                char_idx = 0;
                continue;
            }

            let c = chars[char_idx];
            if c == open {
                depth += 1;
            } else if c == close {
                if depth == 0 {
                    end_pos = Some((line_idx, char_idx));
                    break;
                }
                depth -= 1;
            }

            char_idx += 1;
        } else {
            break;
        }
    }

    end_pos.map(|end| adjust_inner_range(app, start, end))
}

pub fn find_inner_word_range(app: &App) -> Option<((usize, usize), (usize, usize))> {
    word_bounds_at_cursor(app).map(|(start, end)| {
        (
            (app.preview_session.preview.state.cursor_line, start),
            (app.preview_session.preview.state.cursor_line, end),
        )
    })
}

pub fn select_inner_word(app: &mut App) {
    if let Some((start, end)) = word_bounds_at_cursor(app) {
        app.preview_session.preview.state.selection_start =
            Some((app.preview_session.preview.state.cursor_line, start));
        app.preview_session.preview.state.cursor_char = end;
    }
}
