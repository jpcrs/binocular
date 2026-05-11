use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    content: String,
    line_ranges: Vec<(usize, usize)>,
}

impl TextBuffer {
    pub fn new(content: String) -> Self {
        let line_ranges = build_line_ranges(&content);
        Self {
            content,
            line_ranges,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.content
    }

    pub fn len_bytes(&self) -> usize {
        self.content.len()
    }

    pub fn line_ranges(&self) -> &[(usize, usize)] {
        &self.line_ranges
    }

    pub fn line_range(&self, line_idx: usize) -> Option<(usize, usize)> {
        self.line_ranges.get(line_idx).copied()
    }

    pub fn line_count(&self) -> usize {
        self.line_ranges.len()
    }

    pub fn line_slice(&self, line_idx: usize) -> Option<&str> {
        let (start, end) = self.line_range(line_idx)?;
        Some(&self.content[start..end])
    }

    pub fn byte_index(&self, line: usize, char_idx: usize) -> usize {
        let Some((start, end)) = self.line_range(line) else {
            return self.content.len();
        };

        let line_text = &self.content[start..end];
        let mut byte_offset = 0;
        let mut current_char = 0;

        for (idx, c) in line_text.char_indices() {
            if current_char == char_idx {
                return start + idx;
            }
            current_char += 1;
            byte_offset = idx + c.len_utf8();
        }

        if current_char == char_idx {
            start + byte_offset
        } else {
            end
        }
    }

    pub fn line_char_from_byte(&self, byte_idx: usize) -> (usize, usize) {
        let line_idx = self
            .line_ranges
            .iter()
            .position(|(start, end)| byte_idx >= *start && byte_idx <= *end)
            .unwrap_or_else(|| self.line_ranges.len().saturating_sub(1));

        let Some((start, end)) = self.line_range(line_idx) else {
            return (0, 0);
        };
        let clamped = byte_idx.clamp(start, end);
        let char_idx = self.content[start..clamped].chars().count();
        (line_idx, char_idx)
    }

    pub fn line_len_chars(&self, line_idx: usize) -> usize {
        self.line_slice(line_idx)
            .map(|line| {
                line.trim_end_matches('\n')
                    .trim_end_matches('\r')
                    .chars()
                    .count()
            })
            .unwrap_or(0)
    }

    pub fn char_at_byte(&self, byte_idx: usize) -> Option<char> {
        self.content.get(byte_idx..)?.chars().next()
    }

    pub fn char_before_byte(&self, byte_idx: usize) -> Option<(usize, char)> {
        self.content.get(..byte_idx)?.char_indices().last()
    }

    pub fn apply_edit(&mut self, edit: &TextEdit) -> bool {
        match edit.kind {
            TextEditKind::Insert => self.insert(edit.byte_idx, &edit.text),
            TextEditKind::Delete => {
                let end = edit.byte_idx + edit.text.len();
                if self.content.get(edit.byte_idx..end) != Some(edit.text.as_str()) {
                    return false;
                }
                self.delete_range(edit.byte_idx..end).is_some()
            }
        }
    }

    pub fn insert_char(&mut self, byte_idx: usize, c: char) -> Option<TextEdit> {
        let mut text = String::new();
        text.push(c);
        self.insert_text(byte_idx, text)
    }

    pub fn insert_text(&mut self, byte_idx: usize, text: String) -> Option<TextEdit> {
        if !self.insert(byte_idx, &text) {
            return None;
        }
        Some(TextEdit::insert(byte_idx, text))
    }

    pub fn delete_char_before(&mut self, byte_idx: usize) -> Option<TextEdit> {
        let (start, c) = self.char_before_byte(byte_idx)?;
        let end = start + c.len_utf8();
        let deleted = self.delete_range(start..end)?;
        Some(TextEdit::delete(start, deleted))
    }

    pub fn delete_char_at(&mut self, byte_idx: usize) -> Option<TextEdit> {
        let c = self.char_at_byte(byte_idx)?;
        let end = byte_idx + c.len_utf8();
        let deleted = self.delete_range(byte_idx..end)?;
        Some(TextEdit::delete(byte_idx, deleted))
    }

    pub fn delete_range(&mut self, range: Range<usize>) -> Option<String> {
        if range.start > range.end || range.end > self.content.len() {
            return None;
        }
        let removed = self.content.get(range.clone())?.to_string();
        self.content.replace_range(range, "");
        self.rebuild_line_ranges();
        Some(removed)
    }

    fn insert(&mut self, byte_idx: usize, text: &str) -> bool {
        if byte_idx > self.content.len() || !self.content.is_char_boundary(byte_idx) {
            return false;
        }
        self.content.insert_str(byte_idx, text);
        self.rebuild_line_ranges();
        true
    }

    fn rebuild_line_ranges(&mut self) {
        self.line_ranges = build_line_ranges(&self.content);
    }
}

fn build_line_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;

    for (idx, byte) in content.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            ranges.push((start, idx + 1));
            start = idx + 1;
        }
    }

    if start < content.len() {
        ranges.push((start, content.len()));
    } else if start == content.len() && start > 0 {
        ranges.push((start, start));
    }

    if ranges.is_empty() {
        ranges.push((0, 0));
    }

    ranges
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditKind {
    Insert,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub kind: TextEditKind,
    pub byte_idx: usize,
    pub text: String,
}

impl TextEdit {
    pub fn insert(byte_idx: usize, text: String) -> Self {
        Self {
            kind: TextEditKind::Insert,
            byte_idx,
            text,
        }
    }

    pub fn delete(byte_idx: usize, text: String) -> Self {
        Self {
            kind: TextEditKind::Delete,
            byte_idx,
            text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextUndoFrame {
    pub undo_edit: TextEdit,
    pub redo_edit: TextEdit,
    pub before_cursor: (usize, usize),
    pub after_cursor: (usize, usize),
}

impl TextUndoFrame {
    pub fn from_forward_edit(
        edit: TextEdit,
        before_cursor: (usize, usize),
        after_cursor: (usize, usize),
    ) -> Self {
        let inverse = match edit.kind {
            TextEditKind::Insert => TextEdit::delete(edit.byte_idx, edit.text.clone()),
            TextEditKind::Delete => TextEdit::insert(edit.byte_idx, edit.text.clone()),
        };
        Self {
            undo_edit: inverse,
            redo_edit: edit,
            before_cursor,
            after_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_delete_update_line_ranges() {
        let mut buffer = TextBuffer::new("abc\ndef".to_string());
        buffer.insert_char(3, '\n');
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line_slice(1), Some("\n"));

        let deleted = buffer.delete_char_before(4).unwrap();
        assert_eq!(deleted.text, "\n");
        assert_eq!(buffer.as_str(), "abc\ndef");
        assert_eq!(buffer.line_count(), 2);
    }

    #[test]
    fn delete_range_returns_removed_text() {
        let mut buffer = TextBuffer::new("hello world".to_string());
        let removed = buffer.delete_range(5..11).unwrap();
        assert_eq!(removed, " world");
        assert_eq!(buffer.as_str(), "hello");
    }

    #[test]
    fn byte_index_uses_line_context() {
        let buffer = TextBuffer::new("ab\ncd".to_string());
        assert_eq!(buffer.byte_index(1, 1), 4);
        assert_eq!(buffer.line_char_from_byte(4), (1, 1));
    }

    #[test]
    fn delete_range_handles_multi_line_edits() {
        let mut buffer = TextBuffer::new("one\ntwo\nthree".to_string());
        let start = buffer.byte_index(0, 2);
        let end = buffer.byte_index(1, 2);
        let removed = buffer.delete_range(start..end).unwrap();

        assert_eq!(removed, "e\ntw");
        assert_eq!(buffer.as_str(), "ono\nthree");
        assert_eq!(buffer.line_count(), 2);
    }
}
