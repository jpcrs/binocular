use crate::preview::rich_text::TextBuffer;
use ratatui::text::Line;

pub struct RichTextDocument {
    pub buffer: TextBuffer,
    pub lines: Vec<Line<'static>>,
    pub tree: Option<tree_sitter::Tree>,
    /// When true, `lines` and `tree` are stale and need regeneration.
    /// Used during insert mode to skip expensive syntax highlighting.
    pub dirty: bool,
}

impl RichTextDocument {
    pub fn content(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn raw_lines(&self) -> &[(usize, usize)] {
        self.buffer.line_ranges()
    }

    pub fn line_range(&self, line_idx: usize) -> Option<(usize, usize)> {
        self.buffer.line_range(line_idx)
    }

    pub fn line_slice(&self, line_idx: usize) -> Option<&str> {
        self.buffer.line_slice(line_idx)
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn len_bytes(&self) -> usize {
        self.buffer.len_bytes()
    }

    pub fn invalidate_caches(&mut self) {
        self.dirty = true;
        self.tree = None;
    }
}
