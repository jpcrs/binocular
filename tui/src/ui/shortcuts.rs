use crate::app::{InputMode, Mode};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct ShortcutHint {
    pub keys: &'static str,
    pub description: &'static str,
}

const SEARCH_RESULTS_NORMAL_HINTS: &[ShortcutHint] = &[
    ShortcutHint {
        keys: "j/k",
        description: "move",
    },
    ShortcutHint {
        keys: "enter",
        description: "select",
    },
    ShortcutHint {
        keys: "tab",
        description: "mark",
    },
    ShortcutHint {
        keys: "ctrl+w",
        description: "focus preview",
    },
];

const SEARCH_RESULTS_INSERT_HINTS: &[ShortcutHint] = &[
    ShortcutHint {
        keys: "↑/↓",
        description: "move",
    },
    ShortcutHint {
        keys: "enter",
        description: "select",
    },
    ShortcutHint {
        keys: "tab",
        description: "mark",
    },
    ShortcutHint {
        keys: "ctrl+w",
        description: "focus preview",
    },
];

const SEARCH_BAR_NORMAL_HINTS: &[ShortcutHint] = &[
    ShortcutHint {
        keys: "i",
        description: "insert",
    },
    ShortcutHint {
        keys: "h/l",
        description: "move",
    },
    ShortcutHint {
        keys: "w/b",
        description: "word",
    },
    ShortcutHint {
        keys: "j/k",
        description: "list",
    },
];

const SEARCH_BAR_INSERT_HINTS: &[ShortcutHint] = &[ShortcutHint {
    keys: "esc",
    description: "normal",
}];

const PREVIEW_NORMAL_HINTS: &[ShortcutHint] = &[
    ShortcutHint {
        keys: "h/j/k/l",
        description: "move",
    },
    ShortcutHint {
        keys: "i",
        description: "insert",
    },
    ShortcutHint {
        keys: "/",
        description: "find",
    },
    ShortcutHint {
        keys: "ctrl+w",
        description: "focus search",
    },
];

const PREVIEW_INSERT_HINTS: &[ShortcutHint] = &[
    ShortcutHint {
        keys: "esc",
        description: "normal",
    },
    ShortcutHint {
        keys: "ctrl+s",
        description: "save",
    },
    ShortcutHint {
        keys: "enter",
        description: "newline",
    },
];

pub fn search_results_hints(
    app_mode: Mode,
    input_mode: InputMode,
    show_preview: bool,
) -> &'static [ShortcutHint] {
    if app_mode != Mode::Search {
        return &[];
    }

    if !show_preview {
        if input_mode == InputMode::Insert {
            return &SEARCH_RESULTS_INSERT_HINTS[..3];
        }
        return &SEARCH_RESULTS_NORMAL_HINTS[..3];
    }

    if input_mode == InputMode::Insert {
        SEARCH_RESULTS_INSERT_HINTS
    } else {
        SEARCH_RESULTS_NORMAL_HINTS
    }
}

pub fn search_bar_hints(app_mode: Mode, input_mode: InputMode) -> &'static [ShortcutHint] {
    if app_mode != Mode::Search {
        return &[];
    }

    if input_mode == InputMode::Insert {
        SEARCH_BAR_INSERT_HINTS
    } else {
        SEARCH_BAR_NORMAL_HINTS
    }
}

pub fn preview_hints(app_mode: Mode, input_mode: InputMode) -> &'static [ShortcutHint] {
    if app_mode != Mode::Preview {
        return &[];
    }

    if input_mode == InputMode::Insert {
        PREVIEW_INSERT_HINTS
    } else {
        PREVIEW_NORMAL_HINTS
    }
}

pub fn render_hints_line(hints: &[ShortcutHint]) -> Line<'static> {
    let mut spans = Vec::new();

    for (idx, hint) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled("<", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            hint.keys,
            Style::default().fg(Color::LightCyan),
        ));
        spans.push(Span::styled(">", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!(" {}", hint.description),
            Style::default().fg(Color::DarkGray),
        ));
    }

    if !spans.is_empty() {
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}
