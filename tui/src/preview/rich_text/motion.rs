use crate::app::App;

use super::utils::{adjust_scroll, get_line_content, get_line_count, line_len, DocIter};

pub fn move_down(app: &mut App, count: usize) {
    let line_count = get_line_count(app);
    if line_count == 0 {
        return;
    }
    app.preview_session.preview.state.cursor_line =
        (app.preview_session.preview.state.cursor_line + count).min(line_count - 1);
    let max_idx = line_len(app, app.preview_session.preview.state.cursor_line).saturating_sub(1);
    app.preview_session.preview.state.cursor_char =
        app.preview_session.preview.state.cursor_char.min(max_idx);
    adjust_scroll(app);
}

pub fn move_up(app: &mut App, count: usize) {
    let line_count = get_line_count(app);
    if line_count == 0 {
        return;
    }
    app.preview_session.preview.state.cursor_line = app
        .preview_session
        .preview
        .state
        .cursor_line
        .saturating_sub(count);
    let max_idx = line_len(app, app.preview_session.preview.state.cursor_line).saturating_sub(1);
    app.preview_session.preview.state.cursor_char =
        app.preview_session.preview.state.cursor_char.min(max_idx);
    adjust_scroll(app);
}

pub fn move_left(app: &mut App, count: usize) {
    app.preview_session.preview.state.cursor_char = app
        .preview_session
        .preview
        .state
        .cursor_char
        .saturating_sub(count);
}

pub fn move_right(app: &mut App, count: usize) {
    let max_idx = line_len(app, app.preview_session.preview.state.cursor_line).saturating_sub(1);
    app.preview_session.preview.state.cursor_char =
        (app.preview_session.preview.state.cursor_char + count).min(max_idx);
    adjust_scroll(app);
}

pub fn move_start_of_line(app: &mut App) {
    app.preview_session.preview.state.cursor_char = 0;
}

pub fn move_end_of_line(app: &mut App) {
    app.preview_session.preview.state.cursor_char =
        line_len(app, app.preview_session.preview.state.cursor_line).saturating_sub(1);
    adjust_scroll(app);
}

pub fn move_end_of_line_for_insert(app: &mut App) {
    app.preview_session.preview.state.cursor_char =
        line_len(app, app.preview_session.preview.state.cursor_line);
}

pub fn move_half_page_down(app: &mut App, count: usize) {
    let height = if app.preview_height() > 2 {
        app.preview_height() - 2
    } else {
        1
    };
    move_down(app, (height / 2) as usize * count);
}

pub fn move_half_page_up(app: &mut App, count: usize) {
    let height = if app.preview_height() > 2 {
        app.preview_height() - 2
    } else {
        1
    };
    move_up(app, (height / 2) as usize * count);
}

pub fn move_to_end_of_file(app: &mut App, count: usize) {
    let line_count = get_line_count(app);
    if line_count == 0 {
        return;
    }
    if count > 1 {
        app.preview_session.preview.state.cursor_line = (count - 1).min(line_count - 1);
    } else {
        app.preview_session.preview.state.cursor_line = line_count - 1;
    }
}

pub fn move_to_start_of_file(app: &mut App, count: usize) {
    if count > 1 {
        let line_count = get_line_count(app);
        if line_count == 0 {
            return;
        }
        app.preview_session.preview.state.cursor_line = (count - 1).min(line_count - 1);
    } else {
        app.preview_session.preview.state.cursor_line = 0;
    }
}

pub fn move_first_non_blank(app: &mut App) {
    if let Some(content) = get_line_content(app, app.preview_session.preview.state.cursor_line) {
        let chars: Vec<char> = content.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if !c.is_whitespace() {
                app.preview_session.preview.state.cursor_char = i;
                return;
            }
        }
        if !chars.is_empty() {
            app.preview_session.preview.state.cursor_char = chars.len() - 1;
        }
    }
}

pub fn move_word_forward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_word_forward_once(app);
    }
}

pub fn move_word_end_forward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_word_end_forward_once(app);
    }
}

pub fn move_word_backward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_word_backward_once(app);
    }
}

pub fn move_big_word_forward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_big_word_forward_once(app);
    }
}

pub fn move_big_word_end_forward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_big_word_end_forward_once(app);
    }
}

pub fn move_big_word_backward(app: &mut App, count: usize) {
    for _ in 0..count {
        move_big_word_backward_once(app);
    }
}

fn move_word_forward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if it.char_at(app).is_none() {
        return;
    }
    let start_type = it.char_type_at(app);

    // Skip current token type
    while it.char_type_at(app) == start_type {
        if !it.advance(app) {
            break;
        }
    }
    // Skip whitespace
    while it.char_type_at(app) == 0 {
        if !it.advance(app) {
            break;
        }
    }
    it.apply(app);
}

fn move_word_end_forward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if it.char_at(app).is_none() {
        return;
    }

    if !it.advance(app) {
        return;
    }

    while it.char_type_at(app) == 0 {
        if !it.advance(app) {
            break;
        }
    }

    let token_type = it.char_type_at(app);
    loop {
        let mut next = DocIter::new(app, it.line, it.col);
        if !next.advance(app) {
            break;
        }
        if next.char_type_at(app) != token_type {
            break;
        }
        it.line = next.line;
        it.col = next.col;
    }
    it.apply(app);
}

fn move_word_backward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if !it.retreat(app) {
        return;
    }

    while it.char_type_at(app) == 0 {
        if !it.retreat(app) {
            it.apply(app);
            return;
        }
    }

    let token_type = it.char_type_at(app);
    loop {
        let mut prev = DocIter::new(app, it.line, it.col);
        if !prev.retreat(app) {
            break;
        }
        if prev.char_type_at(app) != token_type {
            break;
        }
        it.line = prev.line;
        it.col = prev.col;
    }
    it.apply(app);
}

fn move_big_word_forward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if it.char_at(app).is_none() {
        return;
    }

    while it.char_type_at(app) != 0 {
        if !it.advance(app) {
            break;
        }
    }

    while it.char_type_at(app) == 0 {
        if !it.advance(app) {
            break;
        }
    }
    it.apply(app);
}

fn move_big_word_end_forward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if it.char_at(app).is_none() {
        return;
    }
    if !it.advance(app) {
        return;
    }

    while it.char_type_at(app) == 0 {
        if !it.advance(app) {
            break;
        }
    }

    loop {
        let mut next = DocIter::new(app, it.line, it.col);
        if !next.advance(app) {
            break;
        }
        if next.char_type_at(app) == 0 {
            break;
        }
        it.line = next.line;
        it.col = next.col;
    }
    it.apply(app);
}

fn move_big_word_backward_once(app: &mut App) {
    let mut it = DocIter::from_cursor(app);
    if !it.retreat(app) {
        return;
    }

    while it.char_type_at(app) == 0 {
        if !it.retreat(app) {
            it.apply(app);
            return;
        }
    }

    loop {
        let mut prev = DocIter::new(app, it.line, it.col);
        if !prev.retreat(app) {
            break;
        }
        if prev.char_type_at(app) == 0 {
            break;
        }
        it.line = prev.line;
        it.col = prev.col;
    }
    it.apply(app);
}

pub fn perform_char_search(app: &mut App, target: char, forward: bool, count: usize) {
    if let Some(content) = get_line_content(app, app.preview_session.preview.state.cursor_line) {
        let chars: Vec<char> = content.chars().collect();
        let mut current_idx = app.preview_session.preview.state.cursor_char;

        for _ in 0..count {
            if forward {
                if let Some(idx) = chars
                    .iter()
                    .skip(current_idx + 1)
                    .position(|&c| c == target)
                {
                    current_idx += 1 + idx;
                } else {
                    break;
                }
            } else {
                if current_idx == 0 {
                    break;
                }
                if let Some(idx) = chars[..current_idx].iter().rposition(|&c| c == target) {
                    current_idx = idx;
                } else {
                    break;
                }
            }
        }
        app.preview_session.preview.state.cursor_char = current_idx;
    }
}

pub fn move_matching_bracket(app: &mut App) {
    if let Some(content) = get_line_content(app, app.preview_session.preview.state.cursor_line) {
        let chars: Vec<char> = content.chars().collect();
        if app.preview_session.preview.state.cursor_char >= chars.len() {
            return;
        }

        let mut start_char = chars[app.preview_session.preview.state.cursor_char];

        if !['(', ')', '[', ']', '{', '}'].contains(&start_char) {
            if let Some(idx) = chars
                .iter()
                .skip(app.preview_session.preview.state.cursor_char)
                .position(|&c| ['(', ')', '[', ']', '{', '}'].contains(&c))
            {
                app.preview_session.preview.state.cursor_char += idx;
                start_char = chars[app.preview_session.preview.state.cursor_char];
            } else {
                return;
            }
        }

        let (target_char, forward) = match start_char {
            '(' => (')', true),
            '[' => (']', true),
            '{' => ('}', true),
            ')' => ('(', false),
            ']' => ('[', false),
            '}' => ('{', false),
            _ => return,
        };

        let mut depth = 0;
        let mut line_idx = app.preview_session.preview.state.cursor_line;
        let mut char_idx = app.preview_session.preview.state.cursor_char;
        let line_count = get_line_count(app);

        loop {
            let content = if let Some(s) = get_line_content(app, line_idx) {
                s
            } else {
                break;
            };
            let chars: Vec<char> = content.chars().collect();

            if line_idx == app.preview_session.preview.state.cursor_line
                && char_idx == app.preview_session.preview.state.cursor_char
            {
                if forward {
                    char_idx += 1;
                } else if char_idx > 0 {
                    char_idx -= 1;
                } else if line_idx > 0 {
                    line_idx -= 1;
                    char_idx = usize::MAX;
                } else {
                    break;
                }
            }

            if forward {
                if char_idx >= chars.len() {
                    line_idx += 1;
                    char_idx = 0;
                    if line_idx >= line_count {
                        break;
                    }
                    continue;
                }
            } else {
                if char_idx == usize::MAX {
                    if let Some(s) = get_line_content(app, line_idx) {
                        char_idx = s.chars().count().saturating_sub(1);
                    } else {
                        break;
                    }
                }
                if char_idx >= chars.len() {
                    if !chars.is_empty() {
                        char_idx = chars.len() - 1;
                    } else if line_idx > 0 {
                        line_idx -= 1;
                        char_idx = usize::MAX;
                        continue;
                    } else {
                        break;
                    }
                }
            }

            let c = chars[char_idx];

            if c == target_char {
                if depth == 0 {
                    app.preview_session.preview.state.cursor_line = line_idx;
                    app.preview_session.preview.state.cursor_char = char_idx;
                    return;
                }
                depth -= 1;
            } else if c == start_char {
                depth += 1;
            }

            if forward {
                char_idx += 1;
            } else if char_idx > 0 {
                char_idx -= 1;
            } else if line_idx > 0 {
                line_idx -= 1;
                char_idx = usize::MAX;
            } else {
                break;
            }
        }
    }
}
