use crate::app::InputMode;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub fn mode_indicator(mode: &InputMode, command_buffer: Option<&str>) -> Span<'static> {
    match mode {
        InputMode::Insert => Span::styled(
            " INSERT ",
            Style::default().bg(Color::Green).fg(Color::Black),
        ),
        InputMode::Normal => Span::styled(
            " NORMAL ",
            Style::default().bg(Color::Blue).fg(Color::White),
        ),
        InputMode::Visual => Span::styled(
            " VISUAL ",
            Style::default().bg(Color::Magenta).fg(Color::White),
        ),
        InputMode::VisualLine => Span::styled(
            " V-LINE ",
            Style::default().bg(Color::Magenta).fg(Color::White),
        ),
        InputMode::Command => Span::styled(
            format!(" :{} ", command_buffer.unwrap_or("")),
            Style::default().bg(Color::Yellow).fg(Color::Black),
        ),
    }
}
