use crate::app::{InputMode, Mode};
use crate::ui::indicators::mode_indicator;
use crate::ui::shortcuts::{render_hints_line, search_bar_hints};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::search_border_style;

pub struct SearchBarView<'a> {
    pub app_mode: Mode,
    pub search_mode: InputMode,
    pub preview_search_active: bool,
    pub preview_search_query: &'a str,
    pub preview_search_cursor: usize,
    pub query_text: &'a str,
    pub query_cursor: usize,
    pub search_label: &'a str,
    pub match_mode_label: &'static str,
}

pub fn render_search_bar(f: &mut Frame, view: &SearchBarView<'_>, area: Rect) {
    if view.app_mode == Mode::Preview && view.preview_search_active {
        render_preview_search_bar(f, view, area);
        return;
    }

    render_main_search_bar(f, view, area);
}

fn render_preview_search_bar(f: &mut Frame, view: &SearchBarView<'_>, area: Rect) {
    let input_text = format!("/{}", view.preview_search_query);
    let input = Paragraph::new(input_text.as_str())
        .style(Style::default().fg(Color::Blue))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Search Preview "),
        );
    f.render_widget(input, area);
    f.set_cursor_position((
        area.x + 1 + view.preview_search_cursor as u16 + 1,
        area.y + 1,
    ));
}

fn render_main_search_bar(f: &mut Frame, view: &SearchBarView<'_>, area: Rect) {
    let is_insert_mode = view.search_mode == InputMode::Insert;
    let input_line = build_query_line(view.query_text, view.query_cursor, is_insert_mode);
    let search_mode = if is_insert_mode {
        InputMode::Insert
    } else {
        InputMode::Normal
    };

    let center_title = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            view.search_label,
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ])
    .centered();

    let mode_title = Line::from(vec![Span::raw(" "), mode_indicator(&search_mode, None)]);

    let hints = render_hints_line(search_bar_hints(view.app_mode, view.search_mode));
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(center_title)
        .title(mode_title)
        .title_bottom(hints.right_aligned())
        .border_style(search_border_style(view.app_mode, view.search_mode));
    block = block.title(
        Line::from(vec![Span::styled(
            view.match_mode_label,
            Style::default().add_modifier(Modifier::BOLD),
        )])
        .right_aligned(),
    );
    let input = Paragraph::new(input_line)
        .style(Style::default().fg(Color::White))
        .block(block);

    f.render_widget(input, area);
}

fn build_query_line(query: &str, cursor: usize, is_insert_mode: bool) -> Line<'static> {
    let query_chars: Vec<char> = query.chars().collect();
    let cursor_pos = cursor.min(query_chars.len());

    if query_chars.is_empty() {
        return Line::from(vec![empty_cursor_span(is_insert_mode)]);
    }

    let mut spans = Vec::new();

    if cursor_pos > 0 {
        let before: String = query_chars[..cursor_pos].iter().collect();
        spans.push(Span::raw(before));
    }

    let cursor_char: String = if cursor_pos < query_chars.len() {
        query_chars[cursor_pos..=cursor_pos].iter().collect()
    } else {
        " ".to_string()
    };
    spans.push(cursor_span(cursor_char, is_insert_mode));

    if cursor_pos + 1 < query_chars.len() {
        let after: String = query_chars[cursor_pos + 1..].iter().collect();
        spans.push(Span::raw(after));
    }

    Line::from(spans)
}

fn empty_cursor_span(is_insert_mode: bool) -> Span<'static> {
    if is_insert_mode {
        Span::styled(" ", Style::default().add_modifier(Modifier::UNDERLINED))
    } else {
        Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black))
    }
}

fn cursor_span(content: String, is_insert_mode: bool) -> Span<'static> {
    if is_insert_mode {
        Span::styled(
            content,
            Style::default()
                .add_modifier(Modifier::UNDERLINED)
                .fg(Color::LightGreen),
        )
    } else {
        Span::styled(content, Style::default().bg(Color::White).fg(Color::Black))
    }
}
