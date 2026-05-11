pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

pub fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

pub fn clamp_cursor_insert(cursor: &mut usize, text: &str) {
    *cursor = (*cursor).min(char_count(text));
}

pub fn clamp_cursor_normal(cursor: &mut usize, text: &str) {
    let len = char_count(text);
    if len == 0 {
        *cursor = 0;
    } else if *cursor >= len {
        *cursor = len - 1;
    }
}

pub fn move_right_insert(cursor: &mut usize, text: &str) {
    let len = char_count(text);
    if *cursor < len {
        *cursor += 1;
    }
}

pub fn move_right_normal(cursor: &mut usize, text: &str) {
    let len = char_count(text);
    if *cursor + 1 < len {
        *cursor += 1;
    }
}

pub fn move_word_forward(cursor: &mut usize, text: &str) {
    *cursor = find_next_word_start(text, *cursor);
}

pub fn move_word_end_forward(cursor: &mut usize, text: &str) {
    *cursor = find_word_end(text, *cursor);
}

pub fn move_word_backward(cursor: &mut usize, text: &str) {
    *cursor = find_prev_word_start(text, *cursor);
}

pub fn move_big_word_forward(cursor: &mut usize, text: &str) {
    *cursor = find_next_big_word_start(text, *cursor);
}

pub fn move_big_word_backward(cursor: &mut usize, text: &str) {
    *cursor = find_prev_big_word_start(text, *cursor);
}

pub fn move_start_of_line(cursor: &mut usize) {
    *cursor = 0;
}

pub fn move_end_of_line_normal(cursor: &mut usize, text: &str) {
    let len = char_count(text);
    *cursor = len.saturating_sub(1);
}

pub fn move_end_of_line_insert(cursor: &mut usize, text: &str) {
    *cursor = char_count(text);
}

pub fn move_first_non_blank(cursor: &mut usize, text: &str) {
    *cursor = text.chars().position(|c| !c.is_whitespace()).unwrap_or(0);
}

pub fn insert_char(text: &mut String, cursor: &mut usize, c: char) {
    let byte_idx = char_to_byte_idx(text, *cursor);
    text.insert(byte_idx, c);
    *cursor += 1;
}

pub fn backspace(text: &mut String, cursor: &mut usize) -> bool {
    if *cursor == 0 {
        return false;
    }

    let byte_idx = char_to_byte_idx(text, *cursor - 1);
    text.remove(byte_idx);
    *cursor -= 1;
    true
}

pub fn delete_char_at_cursor(text: &mut String, cursor: usize) -> bool {
    let len = char_count(text);
    if cursor >= len {
        return false;
    }
    let byte_idx = char_to_byte_idx(text, cursor);
    text.remove(byte_idx);
    true
}

pub fn truncate_from_cursor(text: &mut String, cursor: usize) -> bool {
    let len = char_count(text);
    if cursor >= len {
        return false;
    }
    let byte_idx = char_to_byte_idx(text, cursor);
    text.truncate(byte_idx);
    true
}

pub fn replace_char_range(text: &mut String, start_char: usize, end_char: usize) -> bool {
    if start_char >= end_char {
        return false;
    }
    let start_byte = char_to_byte_idx(text, start_char);
    let end_byte = char_to_byte_idx(text, end_char);
    text.replace_range(start_byte..end_byte, "");
    true
}

#[inline]
fn char_at(chars: &[char], i: usize) -> char {
    chars[i]
}

#[inline]
fn byte_at(bytes: &[u8], i: usize) -> char {
    bytes[i] as char
}

/// Thin abstraction over a char sequence that avoids allocation for ASCII.
enum CharSlice<'a> {
    Ascii(&'a [u8]),
    Unicode(Vec<char>),
}

impl<'a> CharSlice<'a> {
    fn new(s: &'a str) -> Self {
        if s.is_ascii() {
            CharSlice::Ascii(s.as_bytes())
        } else {
            CharSlice::Unicode(s.chars().collect())
        }
    }

    fn len(&self) -> usize {
        match self {
            CharSlice::Ascii(b) => b.len(),
            CharSlice::Unicode(v) => v.len(),
        }
    }

    fn get(&self, i: usize) -> char {
        match self {
            CharSlice::Ascii(b) => byte_at(b, i),
            CharSlice::Unicode(v) => char_at(v, i),
        }
    }
}

pub fn find_next_word_start(text: &str, cursor: usize) -> usize {
    let chars = CharSlice::new(text);
    let len = chars.len();
    if cursor >= len {
        return cursor;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let char_type = |c: char| {
        if is_word(c) {
            1u8
        } else if c.is_whitespace() {
            0
        } else {
            2
        }
    };

    let mut i = cursor;
    let start_type = char_type(chars.get(i));
    while i < len && char_type(chars.get(i)) == start_type {
        i += 1;
    }
    while i < len && chars.get(i).is_whitespace() {
        i += 1;
    }

    i.min(len.saturating_sub(1))
}

pub fn find_word_end(text: &str, cursor: usize) -> usize {
    let chars = CharSlice::new(text);
    let len = chars.len();
    if cursor >= len {
        return cursor;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut i = cursor + 1;

    while i < len && chars.get(i).is_whitespace() {
        i += 1;
    }
    if i >= len {
        return len.saturating_sub(1);
    }

    let word_type = is_word(chars.get(i));
    while i + 1 < len {
        let next = chars.get(i + 1);
        if is_word(next) != word_type || next.is_whitespace() {
            break;
        }
        i += 1;
    }

    i.min(len.saturating_sub(1))
}

pub fn find_prev_word_start(text: &str, cursor: usize) -> usize {
    let chars = CharSlice::new(text);
    if cursor == 0 {
        return 0;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut i = cursor.saturating_sub(1);

    while i > 0 && chars.get(i).is_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }

    let word_type = is_word(chars.get(i));
    while i > 0 {
        let prev = chars.get(i - 1);
        if is_word(prev) != word_type || prev.is_whitespace() {
            break;
        }
        i -= 1;
    }

    i
}

pub fn find_next_big_word_start(text: &str, cursor: usize) -> usize {
    let chars = CharSlice::new(text);
    let len = chars.len();
    if cursor >= len {
        return cursor;
    }

    let mut i = cursor;
    while i < len && !chars.get(i).is_whitespace() {
        i += 1;
    }
    while i < len && chars.get(i).is_whitespace() {
        i += 1;
    }

    i.min(len.saturating_sub(1))
}

pub fn find_prev_big_word_start(text: &str, cursor: usize) -> usize {
    let chars = CharSlice::new(text);
    if cursor == 0 {
        return 0;
    }

    let mut i = cursor.saturating_sub(1);
    while i > 0 && chars.get(i).is_whitespace() {
        i -= 1;
    }
    while i > 0 && !chars.get(i - 1).is_whitespace() {
        i -= 1;
    }

    i
}

pub fn find_word_bounds(
    text: &str,
    cursor: usize,
    include_whitespace: bool,
) -> Option<(usize, usize)> {
    let chars = CharSlice::new(text);
    let len = chars.len();
    if cursor >= len {
        return None;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let cur = chars.get(cursor);

    if cur.is_whitespace() {
        let mut start = cursor;
        let mut end = cursor;
        while start > 0 && chars.get(start - 1).is_whitespace() {
            start -= 1;
        }
        while end < len && chars.get(end).is_whitespace() {
            end += 1;
        }
        return Some((start, end));
    }

    let cursor_is_word = is_word(cur);
    let mut start = cursor;
    while start > 0 {
        let prev = chars.get(start - 1);
        if is_word(prev) != cursor_is_word || prev.is_whitespace() {
            break;
        }
        start -= 1;
    }

    let mut end = cursor;
    while end < len {
        let c = chars.get(end);
        if is_word(c) != cursor_is_word || c.is_whitespace() {
            break;
        }
        end += 1;
    }

    if include_whitespace {
        while end < len && chars.get(end).is_whitespace() {
            end += 1;
        }
    }

    Some((start, end))
}

pub fn find_big_word_bounds(
    text: &str,
    cursor: usize,
    include_whitespace: bool,
) -> Option<(usize, usize)> {
    let chars = CharSlice::new(text);
    let len = chars.len();
    if cursor >= len {
        return None;
    }

    if chars.get(cursor).is_whitespace() {
        let mut start = cursor;
        let mut end = cursor;
        while start > 0 && chars.get(start - 1).is_whitespace() {
            start -= 1;
        }
        while end < len && chars.get(end).is_whitespace() {
            end += 1;
        }
        return Some((start, end));
    }

    let mut start = cursor;
    while start > 0 && !chars.get(start - 1).is_whitespace() {
        start -= 1;
    }

    let mut end = cursor;
    while end < len && !chars.get(end).is_whitespace() {
        end += 1;
    }

    if include_whitespace {
        while end < len && chars.get(end).is_whitespace() {
            end += 1;
        }
    }

    Some((start, end))
}
