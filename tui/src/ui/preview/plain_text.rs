use super::PreviewView;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render_plain_text_preview(
    f: &mut Frame,
    area: Rect,
    preview_block: Block<'_>,
    content: &Text<'_>,
    view: &PreviewView<'_>,
) {
    // This paragraph is unwrapped: vertical scrolling is a direct line offset.
    // Borrow only visible spans instead of cloning/walking the entire document.
    let height = preview_block.inner(area).height as usize;
    let visible = Text {
        style: content.style,
        alignment: content.alignment,
        lines: content
            .lines
            .iter()
            .skip(view.scroll as usize)
            .take(height)
            .map(|line| Line {
                style: line.style,
                alignment: line.alignment,
                spans: line
                    .spans
                    .iter()
                    .map(|span| Span::styled(span.content.as_ref(), span.style))
                    .collect(),
            })
            .collect(),
    };
    let paragraph = Paragraph::new(visible)
        .block(preview_block)
        .style(Style::default().bg(Color::Reset))
        .scroll((0, view.scroll_char));
    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{InputMode, Mode};
    use ratatui::{backend::TestBackend, layout::Alignment, widgets::Borders, Terminal};

    #[test]
    fn visible_lines_match_full_paragraph_at_every_scroll_and_size() {
        let text = Text::from(vec![
            Line::from(vec![
                Span::styled("+ café ", Style::default().fg(Color::Green)),
                Span::raw("你好 👩‍💻 e\u{301} long line"),
            ]),
            Line::from("centered").alignment(Alignment::Center),
            Line::from("right aligned")
                .alignment(Alignment::Right)
                .style(Style::default().bg(Color::Blue)),
            Line::default(),
            Line::from("last row"),
        ])
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Right);
        for (width, height) in [(0, 0), (1, 1), (12, 3), (30, 5), (40, 12)] {
            for scroll in [0, 1, 3, 4, 5, u16::MAX] {
                for scroll_char in [0, 1, 8, u16::MAX] {
                    let view = PreviewView {
                        app_mode: Mode::Preview,
                        preview_mode: InputMode::Normal,
                        source: None,
                        status_message: None,
                        command_buffer: None,
                        highlight_line: None,
                        search_query: "",
                        selection_start: None,
                        cursor_line: 0,
                        cursor_char: 0,
                        scroll,
                        scroll_char,
                        area_height: height,
                    };
                    let block = Block::default().borders(Borders::ALL).title("preview");
                    let mut expected = Terminal::new(TestBackend::new(width, height)).unwrap();
                    expected
                        .draw(|f| {
                            f.render_widget(
                                Paragraph::new(text.clone())
                                    .block(block.clone())
                                    .style(Style::default().bg(Color::Reset))
                                    .scroll((scroll, scroll_char)),
                                f.area(),
                            )
                        })
                        .unwrap();
                    let mut actual = Terminal::new(TestBackend::new(width, height)).unwrap();
                    actual
                        .draw(|f| {
                            render_plain_text_preview(f, f.area(), block.clone(), &text, &view)
                        })
                        .unwrap();
                    assert_eq!(
                        actual.backend().buffer(),
                        expected.backend().buffer(),
                        "{width}x{height}, scroll {scroll}/{scroll_char}"
                    );
                }
            }
        }
    }
}
