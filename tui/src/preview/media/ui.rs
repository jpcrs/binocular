use crate::preview::types::MediaPreview;
use crate::ui::preview::PreviewView;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Frame,
};
use ratatui_image::StatefulImage;

pub fn render_media_preview(
    f: &mut Frame,
    area: Rect,
    preview_block: Block<'_>,
    media: &mut MediaPreview,
    view: &PreviewView<'_>,
) {
    let inner = preview_block.inner(area);
    f.render_widget(preview_block, area);

    if let Some(protocol) = &mut media.artwork {
        let min_image_height: u16 = 6;
        let visible_meta_lines = media
            .metadata_line_count
            .saturating_sub(view.scroll as usize) as u16;
        let max_meta_height = inner.height.saturating_sub(min_image_height);
        let meta_height = visible_meta_lines.max(1).min(max_meta_height.max(1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(meta_height), Constraint::Min(0)])
            .split(inner);

        let meta = Paragraph::new(media.metadata.clone())
            .style(Style::default().bg(Color::Reset))
            .scroll((view.scroll, view.scroll_char));
        f.render_widget(meta, chunks[0]);

        let image = StatefulImage::new().resize(ratatui_image::Resize::Fit(None));
        f.render_stateful_widget(image, chunks[1], protocol);
    } else {
        let meta = Paragraph::new(media.metadata.clone())
            .style(Style::default().bg(Color::Reset))
            .scroll((view.scroll, view.scroll_char));
        f.render_widget(meta, inner);
    }
}
