use super::PreviewView;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render_plain_text_preview(
    f: &mut Frame,
    area: Rect,
    preview_block: Block<'_>,
    content: Text<'static>,
    view: &PreviewView<'_>,
) {
    let paragraph = Paragraph::new(content)
        .block(preview_block)
        .style(Style::default().bg(Color::Reset))
        .scroll((view.scroll, view.scroll_char));
    f.render_widget(paragraph, area);
}
