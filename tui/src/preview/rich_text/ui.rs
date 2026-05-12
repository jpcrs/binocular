use crate::preview::RichTextDocument;
use crate::ui::preview::PreviewView;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render_rich_text_preview(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    preview_block: Block<'_>,
    text_file: &RichTextDocument,
    view: &PreviewView<'_>,
) {
    let height = view.area_height.saturating_sub(2) as usize;
    let start_line = view.scroll as usize;
    let end_line = (start_line + height).min(text_file.line_count());

    let rendered = if text_file.dirty {
        crate::preview::generate_plain_lines_for_range(text_file, start_line, end_line)
            .into_iter()
            .enumerate()
            .map(|(offset, line)| transform_line(line, start_line + offset, view))
            .collect::<Vec<_>>()
    } else {
        text_file.lines[start_line..end_line.min(text_file.lines.len())]
            .iter()
            .cloned()
            .enumerate()
            .map(|(offset, line)| transform_line(line, start_line + offset, view))
            .collect::<Vec<_>>()
    };

    let paragraph = Paragraph::new(Text::from(rendered))
        .block(preview_block)
        .style(Style::default().bg(Color::Reset));
    f.render_widget(paragraph, area);
}

fn transform_line(
    mut line: Line<'static>,
    line_idx: usize,
    view: &PreviewView<'_>,
) -> Line<'static> {
    apply_grep_highlight(&mut line, line_idx, view.highlight_line);
    apply_search_highlight(&mut line, view.search_query);
    apply_visual_selection(&mut line, line_idx, view);
    apply_cursor_overlay(&mut line, line_idx, view);
    apply_horizontal_scroll(&mut line, view.scroll_char as usize);
    line
}

fn apply_grep_highlight(line: &mut Line<'static>, line_idx: usize, highlight_line: Option<usize>) {
    let Some(target) = highlight_line else {
        return;
    };

    if line_idx + 1 != target {
        return;
    }

    for span in &mut line.spans {
        span.style = span.style.bg(Color::DarkGray);
    }
    line.style = line.style.bg(Color::DarkGray);
}

fn apply_search_highlight(line: &mut Line<'static>, query: &str) {
    if query.is_empty() {
        return;
    }

    let line_text = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    let matches: Vec<(usize, usize)> = line_text
        .match_indices(query)
        .map(|(idx, matched)| (idx, matched.len()))
        .collect();

    if matches.is_empty() {
        return;
    }

    let mut new_spans = Vec::new();
    let mut current_byte = 0;
    let mut match_idx = 0;

    for span in &line.spans {
        let text = span.content.as_ref();
        let span_len = text.len();
        let span_end = current_byte + span_len;
        let mut local_byte = 0;

        while local_byte < span_len {
            if match_idx >= matches.len() {
                new_spans.push(Span::styled(text[local_byte..].to_string(), span.style));
                break;
            }

            let (m_start, m_len) = matches[match_idx];
            let m_end = m_start + m_len;
            if m_start >= span_end {
                new_spans.push(Span::styled(text[local_byte..].to_string(), span.style));
                break;
            }

            let global = current_byte + local_byte;
            if global < m_start {
                let end_global = m_start.min(span_end);
                let end_local = end_global - current_byte;
                new_spans.push(Span::styled(
                    text[local_byte..end_local].to_string(),
                    span.style,
                ));
                local_byte = end_local;
                continue;
            }

            if global < m_end {
                let end_global = m_end.min(span_end);
                let end_local = end_global - current_byte;
                new_spans.push(Span::styled(
                    text[local_byte..end_local].to_string(),
                    span.style.bg(Color::Yellow).fg(Color::Black),
                ));
                local_byte = end_local;
                if m_end <= span_end {
                    match_idx += 1;
                }
                continue;
            }

            match_idx += 1;
        }

        current_byte += span_len;
    }

    line.spans = new_spans;
}

fn apply_visual_selection(line: &mut Line<'static>, line_idx: usize, view: &PreviewView<'_>) {
    use crate::app::InputMode;

    if view.preview_mode != InputMode::Visual && view.preview_mode != InputMode::VisualLine {
        return;
    }

    let Some((sel_start_line, sel_start_char)) = view.selection_start else {
        return;
    };

    let (sel_end_line, sel_end_char) = (view.cursor_line, view.cursor_char);
    let (start, end) = normalize_selection(
        (sel_start_line, sel_start_char),
        (sel_end_line, sel_end_char),
    );

    if line_idx < start.0 || line_idx > end.0 {
        return;
    }

    if view.preview_mode == InputMode::VisualLine {
        apply_visual_line_selection(line);
    } else {
        apply_visual_char_selection(line, line_idx, start, end);
    }
}

fn normalize_selection(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) {
        (a, b)
    } else {
        (b, a)
    }
}

fn apply_visual_line_selection(line: &mut Line<'static>) {
    let line_num_width = line_number_width(line);
    let mut current_len = 0usize;
    let mut spans = Vec::new();

    for span in &line.spans {
        if current_len < line_num_width {
            spans.push(span.clone());
        } else {
            spans.push(Span::styled(
                span.content.to_string(),
                span.style.bg(Color::LightBlue).fg(Color::Black),
            ));
        }
        current_len += span.content.chars().count();
    }

    line.spans = spans;
}

fn apply_visual_char_selection(
    line: &mut Line<'static>,
    line_idx: usize,
    start: (usize, usize),
    end: (usize, usize),
) {
    let line_num_width = line_number_width(line);
    let mut current_len = 0usize;
    let mut spans = Vec::new();

    for span in &line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let span_len = chars.len();
        let span_end = current_len + span_len;

        let highlight_start = if line_idx == start.0 {
            start.1 + line_num_width
        } else {
            line_num_width
        };
        let highlight_end = if line_idx == end.0 {
            end.1 + 1 + line_num_width
        } else {
            usize::MAX
        };

        let intersect_start = current_len.max(highlight_start);
        let intersect_end = span_end.min(highlight_end);

        if intersect_start < intersect_end {
            let local_start = intersect_start - current_len;
            let local_end = intersect_end - current_len;

            if local_start > 0 {
                spans.push(Span::styled(
                    chars[..local_start].iter().collect::<String>(),
                    span.style,
                ));
            }
            spans.push(Span::styled(
                chars[local_start..local_end].iter().collect::<String>(),
                span.style.bg(Color::LightBlue).fg(Color::Black),
            ));
            if local_end < span_len {
                spans.push(Span::styled(
                    chars[local_end..].iter().collect::<String>(),
                    span.style,
                ));
            }
        } else {
            spans.push(span.clone());
        }

        current_len += span_len;
    }

    line.spans = spans;
}

fn apply_cursor_overlay(line: &mut Line<'static>, line_idx: usize, view: &PreviewView<'_>) {
    use crate::app::InputMode;

    if line_idx != view.cursor_line {
        return;
    }

    if view.preview_mode == InputMode::Insert {
        apply_insert_cursor(line, view.cursor_char);
    } else if view.preview_mode == InputMode::Normal {
        apply_normal_cursor(line, view.cursor_char);
    }
}

fn apply_insert_cursor(line: &mut Line<'static>, cursor_char: usize) {
    let cursor_pos = cursor_char + line_number_width(line);
    let mut current_len = 0usize;
    let mut spans = Vec::new();
    let mut inserted = false;

    for span in &line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let span_len = chars.len();

        if !inserted && cursor_pos >= current_len && cursor_pos <= current_len + span_len {
            let local = cursor_pos - current_len;
            if local > 0 {
                spans.push(Span::styled(
                    chars[..local].iter().collect::<String>(),
                    span.style,
                ));
            }
            spans.push(Span::styled("▌", span.style.fg(Color::LightGreen)));
            if local < span_len {
                spans.push(Span::styled(
                    chars[local..].iter().collect::<String>(),
                    span.style,
                ));
            }
            inserted = true;
        } else {
            spans.push(span.clone());
        }

        current_len += span_len;
    }

    if !inserted {
        spans.push(Span::styled("▌", Style::default().fg(Color::LightGreen)));
    }

    line.spans = spans;
}

fn apply_normal_cursor(line: &mut Line<'static>, cursor_char: usize) {
    let cursor_pos = cursor_char + line_number_width(line);
    let mut current_len = 0usize;
    let mut spans = Vec::new();
    let mut applied = false;

    for span in &line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let span_len = chars.len();

        if !applied && cursor_pos >= current_len && cursor_pos < current_len + span_len {
            let local = cursor_pos - current_len;
            if local > 0 {
                spans.push(Span::styled(
                    chars[..local].iter().collect::<String>(),
                    span.style,
                ));
            }
            spans.push(Span::styled(
                chars[local].to_string(),
                span.style.bg(Color::White).fg(Color::Black),
            ));
            if local + 1 < span_len {
                spans.push(Span::styled(
                    chars[local + 1..].iter().collect::<String>(),
                    span.style,
                ));
            }
            applied = true;
        } else {
            spans.push(span.clone());
        }

        current_len += span_len;
    }

    if !applied {
        spans.push(Span::styled(
            " ",
            Style::default().bg(Color::White).fg(Color::Black),
        ));
    }

    line.spans = spans;
}

fn apply_horizontal_scroll(line: &mut Line<'static>, scroll: usize) {
    if scroll == 0 {
        return;
    }

    let mut current = 0usize;
    let mut spans = Vec::new();

    for span in &line.spans {
        let span_len = span.content.chars().count();
        if current + span_len <= scroll {
            current += span_len;
            continue;
        }

        if current < scroll {
            let offset = scroll - current;
            let cropped: String = span.content.chars().skip(offset).collect();
            spans.push(Span::styled(cropped, span.style));
        } else {
            spans.push(span.clone());
        }

        current += span_len;
    }

    line.spans = spans;
}

fn line_number_width(line: &Line<'_>) -> usize {
    line.spans
        .first()
        .map(|span| span.content.chars().count())
        .unwrap_or(0)
}
