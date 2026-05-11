mod plain_text;

use crate::app::{InputMode, Mode};
use crate::preview::{self, PreviewContent, PreviewSource};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::ui::indicators::mode_indicator;
use crate::ui::shortcuts::{preview_hints, render_hints_line};

pub struct PreviewView<'a> {
    pub app_mode: Mode,
    pub preview_mode: InputMode,
    pub source: Option<&'a PreviewSource>,
    pub status_message: Option<(&'a str, std::time::Instant)>,
    pub command_buffer: Option<&'a str>,
    pub highlight_line: Option<usize>,
    pub search_query: &'a str,
    pub selection_start: Option<(usize, usize)>,
    pub cursor_line: usize,
    pub cursor_char: usize,
    pub scroll: u16,
    pub scroll_char: u16,
    pub area_height: u16,
}

pub fn render_preview(
    f: &mut Frame,
    view: &PreviewView<'_>,
    content: Option<&mut PreviewContent>,
    area: Rect,
) {
    let is_read_only = is_read_only_preview(content.as_deref());
    let preview_style = if is_read_only {
        Style::default().fg(Color::Yellow)
    } else if view.app_mode == Mode::Preview {
        Style::default().fg(Color::Blue)
    } else {
        Style::default()
    };

    let status_line = build_status_line(view, content.as_deref());
    let hints = render_hints_line(preview_hints(view.app_mode, view.preview_mode));
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(build_preview_title(view.source).centered())
        .title_bottom(status_line)
        .title_bottom(hints.right_aligned())
        .border_style(preview_style);

    f.render_widget(ratatui::widgets::Clear, area);

    match content {
        Some(PreviewContent::RichText(text_file)) => {
            preview::rich_text::ui::render_rich_text_preview(
                f,
                area,
                preview_block,
                text_file,
                view,
            );
        }
        Some(PreviewContent::Diff(diff)) => {
            plain_text::render_plain_text_preview(f, area, preview_block, diff.text.clone(), view);
        }
        Some(PreviewContent::PlainText(content)) => {
            plain_text::render_plain_text_preview(f, area, preview_block, content.clone(), view);
        }
        Some(PreviewContent::Image(image)) => {
            preview::image::ui::render_image_preview(f, area, preview_block, image, view);
        }
        Some(PreviewContent::Media(media)) => {
            preview::media::ui::render_media_preview(f, area, preview_block, media, view);
        }
        Some(PreviewContent::StructuredLog(lp)) => {
            let inner = preview_block.inner(area);
            f.render_widget(preview_block, area);
            preview::structured_log::ui::render_structured_log(f, inner, lp);
        }
        None => {
            f.render_widget(preview_block, area);
        }
    }
}

fn build_preview_title(source: Option<&PreviewSource>) -> Line<'static> {
    let Some(source) = source else {
        return Line::from("Preview");
    };
    let title = source.title();
    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            shorten_path(title.as_ref()),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ])
}

/// Show `…/parent/name` for deep paths, or just `name` for shallow ones.
fn shorten_path(path: &str) -> String {
    use std::path::Path;
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let p = Path::new(stripped);
    let name = match p.file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => return stripped.to_string(),
    };
    match p.parent().filter(|par| *par != Path::new("")) {
        None => name,
        Some(parent) => {
            let parent_name = parent
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| parent.to_string_lossy().into_owned());
            format!("{}/{}", parent_name, name)
        }
    }
}

fn build_status_line(view: &PreviewView<'_>, content: Option<&PreviewContent>) -> Line<'static> {
    if is_read_only_preview(content) {
        return Line::from(vec![Span::styled(
            " READ-ONLY ",
            Style::default().fg(Color::Yellow),
        )]);
    }

    if let Some((msg, time)) = view.status_message {
        if time.elapsed().as_secs() < 3 {
            return Line::from(vec![Span::styled(
                format!(" {} ", msg),
                Style::default().fg(Color::Green),
            )]);
        }
    }

    Line::from(vec![
        Span::raw(" "),
        mode_indicator(&view.preview_mode, view.command_buffer),
    ])
}

fn is_read_only_preview(content: Option<&PreviewContent>) -> bool {
    matches!(
        content,
        Some(PreviewContent::Diff(_))
            | Some(PreviewContent::PlainText(_))
            | Some(PreviewContent::Image(_))
            | Some(PreviewContent::Media(_))
    )
}
