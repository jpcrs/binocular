use crate::app::App;
use crate::preview::PreviewContent;

pub fn get_line_content(app: &App, line_idx: usize) -> Option<String> {
    if let Some(PreviewContent::RichText(text)) = &app.preview_session.preview.content {
        return text.line_slice(line_idx).map(ToString::to_string);
    }
    None
}

pub fn get_line_count(app: &App) -> usize {
    if let Some(PreviewContent::RichText(text)) = &app.preview_session.preview.content {
        text.line_count()
    } else {
        0
    }
}

pub fn get_byte_index(content: &str, line: usize, char_idx: usize) -> usize {
    let mut current_line = 0;
    let mut current_char = 0;
    let mut byte_idx = 0;

    for (idx, c) in content.char_indices() {
        if current_line == line && current_char == char_idx {
            return idx;
        }

        if c == '\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }
        byte_idx = idx + c.len_utf8();
    }

    if current_line == line && current_char == char_idx {
        return byte_idx;
    }

    byte_idx
}

pub fn get_line_char_from_byte(content: &str, byte_idx: usize) -> (usize, usize) {
    let mut line = 0;
    let mut char_idx = 0;
    let mut current_byte = 0;

    for (idx, c) in content.char_indices() {
        if idx == byte_idx {
            return (line, char_idx);
        }
        if c == '\n' {
            line += 1;
            char_idx = 0;
        } else {
            char_idx += 1;
        }
        current_byte = idx + c.len_utf8();
    }

    if current_byte == byte_idx {
        return (line, char_idx);
    }

    (line, char_idx)
}

pub fn line_len(app: &App, line_idx: usize) -> usize {
    if let Some(content) = get_line_content(app, line_idx) {
        let chars: Vec<char> = content.chars().collect();
        let mut len = chars.len();
        while len > 0 && (chars[len - 1] == '\n' || chars[len - 1] == '\r') {
            len -= 1;
        }
        len
    } else {
        0
    }
}

pub fn char_type(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' {
        2
    } else if c.is_whitespace() {
        0
    } else {
        1
    }
}

pub struct DocIter {
    pub line: usize,
    pub col: usize,
    line_count: usize,
}

impl DocIter {
    pub fn new(app: &App, line: usize, col: usize) -> Self {
        Self {
            line,
            col,
            line_count: get_line_count(app),
        }
    }

    pub fn from_cursor(app: &App) -> Self {
        Self::new(
            app,
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        )
    }

    pub fn char_at(&self, app: &App) -> Option<char> {
        get_line_content(app, self.line).and_then(|s| s.chars().nth(self.col))
    }

    pub fn char_type_at(&self, app: &App) -> u8 {
        self.char_at(app).map(char_type).unwrap_or(0)
    }

    pub fn advance(&mut self, app: &App) -> bool {
        let len = get_line_content(app, self.line)
            .map(|s| s.chars().count())
            .unwrap_or(0);
        if self.col + 1 < len {
            self.col += 1;
            true
        } else if self.line + 1 < self.line_count {
            self.line += 1;
            self.col = 0;
            true
        } else {
            false
        }
    }

    pub fn retreat(&mut self, app: &App) -> bool {
        if self.col > 0 {
            self.col -= 1;
            true
        } else if self.line > 0 {
            self.line -= 1;
            let len = get_line_content(app, self.line)
                .map(|s| s.chars().count())
                .unwrap_or(0);
            self.col = if len > 0 { len - 1 } else { 0 };
            true
        } else {
            false
        }
    }

    pub fn apply(&self, app: &mut App) {
        app.preview_session.preview.state.cursor_line = self.line;
        app.preview_session.preview.state.cursor_char = self.col;
    }
}

pub fn ensure_cursor_in_bounds(app: &mut App) {
    use crate::app::InputMode;

    let line_count = get_line_count(app);
    if line_count == 0 {
        app.preview_session.preview.state.cursor_line = 0;
        app.preview_session.preview.state.cursor_char = 0;
        return;
    }

    if app.preview_session.preview.state.cursor_line >= line_count {
        app.preview_session.preview.state.cursor_line = line_count - 1;
    }

    let len = line_len(app, app.preview_session.preview.state.cursor_line);
    if len == 0 {
        app.preview_session.preview.state.cursor_char = 0;
    } else if app.preview_session.preview.state.mode == InputMode::Insert {
        if app.preview_session.preview.state.cursor_char > len {
            app.preview_session.preview.state.cursor_char = len;
        }
    } else if app.preview_session.preview.state.cursor_char >= len {
        app.preview_session.preview.state.cursor_char = len - 1;
    }
}

pub fn adjust_scroll(app: &mut App) {
    let viewport_height = if app.preview_height() > 2 {
        app.preview_height() - 2
    } else {
        1
    };
    let viewport_width = if app.preview_width() > 2 {
        app.preview_width() - 2
    } else {
        1
    };

    const LINE_NUM_WIDTH: usize = 5;

    if app.preview_session.preview.state.cursor_line < app.preview_session.preview.state.scroll {
        app.preview_session.preview.state.scroll = app.preview_session.preview.state.cursor_line;
    } else if app.preview_session.preview.state.cursor_line
        >= (app.preview_session.preview.state.scroll + viewport_height as usize)
    {
        app.preview_session.preview.state.scroll = (app.preview_session.preview.state.cursor_line
            + 1)
        .saturating_sub(viewport_height as usize);
    }

    let content_viewport = (viewport_width as usize).saturating_sub(LINE_NUM_WIDTH);

    if app.preview_session.preview.state.cursor_char < app.preview_session.preview.state.scroll_char
    {
        app.preview_session.preview.state.scroll_char =
            app.preview_session.preview.state.cursor_char;
    }

    let visible_end = app.preview_session.preview.state.scroll_char + content_viewport;
    if app.preview_session.preview.state.cursor_char >= visible_end {
        app.preview_session.preview.state.scroll_char =
            (app.preview_session.preview.state.cursor_char + 1).saturating_sub(content_viewport);
    }
}
