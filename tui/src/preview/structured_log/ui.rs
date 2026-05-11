use crate::preview::structured_log::ColumnConfig;
use crate::preview::types::LogPreview;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

const COL_SEP: &str = "  ";
const COL_SEP_LEN: usize = COL_SEP.len();

fn num_width(total: usize) -> usize {
    if total == 0 {
        1
    } else {
        total.to_string().len()
    }
}

fn prefix_width(total: usize) -> usize {
    num_width(total) + 3
}

pub fn render_structured_log(f: &mut Frame, area: Rect, lp: &mut LogPreview) {
    if lp.filter_state.visible_cols.is_empty() {
        f.render_widget(
            Paragraph::new("No columns configured. Press 'a' to add columns."),
            area,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let pfx = prefix_width(lp.log.entries.len());
    let data_width = (chunks[3].width as usize).saturating_sub(pfx);
    let col_scroll = compute_col_scroll(
        &lp.filter_state.visible_cols,
        lp.filter_state.selected_col,
        lp.filter_state.col_scroll,
        data_width,
    );

    let (start, rendered) =
        visible_col_range(&lp.filter_state.visible_cols, col_scroll, data_width);
    let total_cols = lp.filter_state.visible_cols.len();

    render_filter_bar(f, chunks[0], lp, start, start + rendered, total_cols);
    render_header(f, chunks[1], lp, start, rendered);
    render_separator(f, chunks[2], lp, start, rendered);
    render_rows(f, chunks[3], lp, start, rendered);

    if lp.filter_state.col_modal.is_some() {
        render_col_modal(f, area, lp);
    }
}

fn compute_col_scroll(
    cols: &[ColumnConfig],
    selected: usize,
    current_scroll: usize,
    area_width: usize,
) -> usize {
    if cols.is_empty() || area_width == 0 {
        return 0;
    }
    let selected = selected.min(cols.len() - 1);
    let mut scroll = current_scroll.min(selected);

    loop {
        let mut used = 0usize;
        let mut last_visible = scroll;
        for (rel_i, col) in cols[scroll..].iter().enumerate() {
            let abs_i = scroll + rel_i;
            let sep = if rel_i > 0 { COL_SEP_LEN } else { 0 };
            let needed = col.width + sep;
            if used + needed > area_width && rel_i > 0 {
                break;
            }
            used += needed;
            last_visible = abs_i;
        }
        if last_visible >= selected {
            return scroll;
        }
        scroll += 1;
    }
}

fn visible_col_range(
    cols: &[ColumnConfig],
    col_scroll: usize,
    area_width: usize,
) -> (usize, usize) {
    let start = col_scroll.min(cols.len().saturating_sub(1));
    let mut used = 0usize;
    let mut count = 0usize;
    for (rel_i, col) in cols[start..].iter().enumerate() {
        let sep = if rel_i > 0 { COL_SEP_LEN } else { 0 };
        let needed = col.width + sep;
        if used + needed > area_width && count > 0 {
            break;
        }
        used += needed;
        count += 1;
    }
    (start, count)
}

fn render_filter_bar(
    f: &mut Frame,
    area: Rect,
    lp: &LogPreview,
    col_start: usize,
    col_end: usize,
    total_cols: usize,
) {
    let fs = &lp.filter_state;
    let matched = fs.cached_matches.len();
    let total = lp.log.entries.len();

    let mark_str = if !fs.marked.is_empty() {
        format!(" ●{} ", fs.marked.len())
    } else {
        String::new()
    };
    let hidden_left = col_start;
    let hidden_right = total_cols.saturating_sub(col_end);
    let col_info = if hidden_left > 0 || hidden_right > 0 {
        format!(
            " ←{} col {}/{} →{}  ",
            hidden_left,
            fs.selected_col + 1,
            total_cols,
            hidden_right
        )
    } else {
        format!(" col {}/{}  ", fs.selected_col + 1, total_cols)
    };
    let paused_str = if fs.paused { " ⏸ PAUSED " } else { "" };
    let count_str = format!(" {}/{} ", matched, total);
    let right_str = format!("{}{}{}{}", paused_str, mark_str, col_info, count_str);
    let right_width = right_str.len() as u16;
    let left_width = area.width.saturating_sub(right_width);

    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width),
            Constraint::Length(right_width),
        ])
        .split(area);

    let cursor_span = if fs.input_active {
        Span::styled("▌", Style::default().fg(Color::Blue))
    } else {
        Span::raw("")
    };
    let input_style = if fs.input_active {
        Style::default().fg(Color::Blue)
    } else if !fs.input.is_empty() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let filter_line = Line::from(vec![
        Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
        Span::styled(fs.input.clone(), input_style),
        cursor_span,
    ]);

    let mark_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let col_style = if hidden_left > 0 || hidden_right > 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let count_style = if matched < total && !fs.input.is_empty() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut right_spans: Vec<Span> = Vec::new();
    if fs.paused {
        right_spans.push(Span::styled(
            paused_str,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    if !mark_str.is_empty() {
        right_spans.push(Span::styled(mark_str, mark_style));
    }
    right_spans.push(Span::styled(col_info, col_style));
    right_spans.push(Span::styled(count_str, count_style));
    let right_line = Line::from(right_spans);

    f.render_widget(Paragraph::new(filter_line), bar_chunks[0]);
    f.render_widget(Paragraph::new(right_line), bar_chunks[1]);
}

fn render_header(f: &mut Frame, area: Rect, lp: &LogPreview, start: usize, count: usize) {
    let fs = &lp.filter_state;
    let cols = &fs.visible_cols;

    let mut spans: Vec<Span> = vec![Span::raw(" ".repeat(prefix_width(lp.log.entries.len())))];
    spans.extend(
        cols.iter()
            .enumerate()
            .skip(start)
            .take(count)
            .flat_map(|(i, col)| {
                let is_selected = i == fs.selected_col;
                let label = truncate_str(&col.field.to_ascii_uppercase(), col.width);
                let padded = pad_right(&label, col.width);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                };
                let span = Span::styled(padded, style);
                if i < start + count - 1 {
                    vec![span, Span::raw(COL_SEP)]
                } else {
                    vec![span]
                }
            }),
    );

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_separator(f: &mut Frame, area: Rect, lp: &LogPreview, start: usize, count: usize) {
    let fs = &lp.filter_state;
    let cols = &fs.visible_cols;

    let mut spans: Vec<Span> = vec![Span::raw(" ".repeat(prefix_width(lp.log.entries.len())))];
    for (i, col) in cols.iter().enumerate().skip(start).take(count) {
        let is_selected = i == fs.selected_col;
        let dashes = "─".repeat(col.width);
        let style = if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(dashes, style));
        if i < start + count - 1 {
            spans.push(Span::raw(COL_SEP));
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_rows(f: &mut Frame, area: Rect, lp: &mut LogPreview, start: usize, count: usize) {
    let visible_rows = area.height as usize;
    if visible_rows == 0 || count == 0 {
        return;
    }

    let n_matches = lp.filter_state.cached_matches.len();
    if n_matches > 0 {
        let cursor = lp.filter_state.cursor.min(n_matches - 1);
        lp.filter_state.cursor = cursor;
        let scroll = lp.filter_state.scroll;
        lp.filter_state.scroll = if cursor < scroll {
            cursor
        } else if cursor >= scroll + visible_rows {
            cursor + 1 - visible_rows
        } else {
            scroll
        };
    }

    let fs = &lp.filter_state;
    let entries = &lp.log.entries;
    let cols = &fs.visible_cols;
    let cursor = fs.cursor;
    let n_width = num_width(entries.len());

    let lines: Vec<Line> = fs
        .cached_matches
        .iter()
        .enumerate()
        .skip(fs.scroll)
        .take(visible_rows)
        .map(|(match_idx, &entry_idx)| {
            let entry = &entries[entry_idx];
            let is_selected_row = match_idx == cursor;
            let is_marked = fs.marked.contains(&entry_idx);

            let row_bg = if is_selected_row {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let lnum = format!("{:>width$}", entry_idx + 1, width = n_width);
            let lnum_style = if is_selected_row {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let (indicator, ind_style) = if is_selected_row && is_marked {
                (
                    "●",
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected_row {
                ("▶", Style::default().fg(Color::Cyan).bg(Color::DarkGray))
            } else if is_marked {
                ("●", Style::default().fg(Color::Yellow))
            } else {
                (" ", Style::default())
            };

            let mut spans: Vec<Span> = vec![
                Span::styled(lnum, lnum_style),
                Span::styled(" ", row_bg),
                Span::styled(indicator, ind_style),
                Span::styled(" ", row_bg),
            ];

            spans.extend(cols.iter().enumerate().skip(start).take(count).flat_map(
                |(col_i, col)| {
                    let val = entry
                        .fields
                        .iter()
                        .find(|(k, _)| k.as_str() == col.field.as_str())
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("");

                    let display_val = if is_timestamp_field(&col.field) {
                        abbreviate_timestamp(val)
                    } else {
                        val.to_string()
                    };

                    let truncated = truncate_str(&display_val, col.width);
                    let padded = pad_right(&truncated, col.width);

                    let style = cell_style(&col.field, val, is_selected_row);
                    let span = Span::styled(padded, style);

                    if col_i < start + count - 1 {
                        vec![span, Span::styled(COL_SEP, row_bg)]
                    } else {
                        vec![span]
                    }
                },
            ));

            Line::from(spans)
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}

fn render_col_modal(f: &mut Frame, area: Rect, lp: &mut LogPreview) {
    let Some(modal) = &lp.filter_state.col_modal else {
        return;
    };
    let all_fields = &lp.log.all_fields;
    let n = all_fields.len();

    let max_label_width = all_fields
        .iter()
        .map(|f| f.chars().count())
        .max()
        .unwrap_or(8);
    let modal_inner_width = (max_label_width + 6).max(36).min(area.width as usize - 4);
    let modal_inner_height = (n + 4).min(area.height.saturating_sub(4) as usize).max(6);
    let modal_w = (modal_inner_width + 2) as u16;
    let modal_h = (modal_inner_height + 2) as u16;

    let mx = area.x + area.width.saturating_sub(modal_w) / 2;
    let my = area.y + area.height.saturating_sub(modal_h) / 2;
    let modal_rect = Rect {
        x: mx,
        y: my,
        width: modal_w,
        height: modal_h,
    };

    f.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(
            " Column Visibility ",
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " Space/Tab toggle  j/k move  Enter apply  Esc cancel ",
            Style::default().fg(Color::DarkGray),
        ));
    f.render_widget(block.clone(), modal_rect);
    let inner = block.inner(modal_rect);

    let list_height = inner.height as usize;
    let cursor = modal.cursor.min(n.saturating_sub(1));
    let scroll = if cursor >= list_height {
        cursor - list_height + 1
    } else {
        0
    };

    let lines: Vec<Line> = all_fields
        .iter()
        .enumerate()
        .skip(scroll)
        .take(list_height)
        .map(|(i, field)| {
            let checked = modal.checked.get(i).copied().unwrap_or(false);
            let is_cursor = i == cursor;
            let checkbox = if checked { "[✓] " } else { "[ ] " };
            let text = format!("{}{}", checkbox, field);
            let style = if is_cursor {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if checked {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

const TIMESTAMP_FIELDS: &[&str] = &["time", "timestamp", "ts", "datetime", "date", "@timestamp"];
const LEVEL_FIELDS: &[&str] = &["level", "severity", "lvl", "log_level", "loglevel"];

fn is_timestamp_field(field: &str) -> bool {
    let lower = field.to_ascii_lowercase();
    TIMESTAMP_FIELDS.iter().any(|c| *c == lower.as_str())
}

fn abbreviate_timestamp(value: &str) -> String {
    let v = value.trim();
    if let Some(tpos) = v.find('T') {
        let after = &v[tpos + 1..];
        let after = after.trim_end_matches('Z');
        let main = after.split('+').next().unwrap_or(after);
        let main = if main.len() > 8 {
            &main[..12.min(main.len())]
        } else {
            main
        };
        return main.to_string();
    }
    if v.len() >= 19 && v.as_bytes().get(10) == Some(&b' ') {
        return v[11..19].to_string();
    }
    v.to_string()
}

fn cell_style(field: &str, value: &str, selected: bool) -> Style {
    let base = if selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    let lower_field = field.to_ascii_lowercase();
    if LEVEL_FIELDS.iter().any(|f| *f == lower_field.as_str()) {
        let lower = value.to_ascii_lowercase();
        if ["error", "err", "fatal", "critical"].contains(&lower.as_str()) {
            return base.fg(Color::Red).add_modifier(Modifier::BOLD);
        }
        if ["warn", "warning"].contains(&lower.as_str()) {
            return base.fg(Color::Yellow).add_modifier(Modifier::BOLD);
        }
        if ["info"].contains(&lower.as_str()) {
            return base.fg(Color::Green);
        }
        if ["debug", "trace"].contains(&lower.as_str()) {
            return base.fg(Color::Cyan);
        }
    }

    if is_timestamp_field(field) {
        return base.fg(Color::DarkGray);
    }

    base
}

fn truncate_str(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    chars[..width - 1].iter().collect::<String>() + "…"
}

fn pad_right(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}
