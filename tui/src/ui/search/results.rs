use crate::app::{InputMode, Mode};
use crate::search::sources::git::{is_current_commit, HISTORY_PATH_SEPARATOR};
use crate::search::types::{SearchItem, SearchResult};
use crate::text::truncate_str_chars;
use crate::ui::search::search_border_style;
use crate::ui::shortcuts::{render_hints_line, search_results_hints};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState},
    Frame,
};
use std::borrow::Cow;
use std::collections::HashMap;

pub struct SearchResultsView<'a> {
    pub app_mode: Mode,
    pub query_mode: InputMode,
    pub show_preview: bool,
    pub is_content_mode: bool,
    pub stdin_mode: bool,
    pub query_is_empty: bool,
    pub total_matches: u64,
    pub total_items: u64,
    pub working: bool,
    pub marked_count: usize,
    pub diff_marked_count: usize,
    pub results: &'a [SearchResult],
    pub marked_items: &'a HashMap<SearchItem, Option<usize>>,
    pub diff_marked_items: &'a std::collections::HashSet<SearchItem>,
}

pub fn render_search_results(
    f: &mut Frame,
    view: &SearchResultsView<'_>,
    scroll_state: &mut ListState,
    area: Rect,
) {
    let visible_rows = area.height.saturating_sub(2) as usize;
    let (start, end, selected) = visible_result_range(scroll_state, view.results.len(), visible_rows);
    let items: Vec<ListItem> = view
        .results
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|result| build_result_item(result, view))
        .collect();

    let border_style = search_border_style(view.app_mode, view.query_mode);

    let hints = render_hints_line(search_results_hints(
        view.app_mode,
        view.query_mode,
        view.show_preview,
    ));

    let is_filtering = !view.query_is_empty;
    let count_badge = build_count_badge(
        view.total_matches,
        view.total_items,
        view.marked_count,
        view.diff_marked_count,
        view.working,
        is_filtering,
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(
                    Line::from(vec![
                        Span::raw(" "),
                        Span::styled("Results", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                    ])
                    .centered(),
                )
                .title_top(count_badge.right_aligned())
                .title_bottom(hints.right_aligned())
                .border_style(border_style),
        )
        .style(Style::default().bg(Color::Reset))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▌ ")
        .highlight_spacing(HighlightSpacing::Always);

    let mut visible_state = ListState::default().with_selected(selected);
    f.render_stateful_widget(list, area, &mut visible_state);
}

fn visible_result_range(
    scroll_state: &mut ListState,
    result_count: usize,
    visible_rows: usize,
) -> (usize, usize, Option<usize>) {
    if result_count == 0 || visible_rows == 0 {
        *scroll_state.offset_mut() = 0;
        return (0, 0, None);
    }

    let selected = scroll_state.selected().unwrap_or(0).min(result_count - 1);
    let max_offset = result_count.saturating_sub(visible_rows);
    let mut offset = scroll_state.offset().min(max_offset);

    if selected < offset {
        offset = selected;
    } else if selected >= offset + visible_rows {
        offset = selected + 1 - visible_rows;
    }

    *scroll_state.offset_mut() = offset;
    let end = (offset + visible_rows).min(result_count);

    (offset, end, Some(selected - offset))
}

fn build_count_badge(
    total: u64,
    total_items: u64,
    marked: usize,
    diff_marked: usize,
    working: bool,
    _is_filtering: bool,
) -> Line<'static> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = Vec::new();

    spans.push(Span::raw(" "));
    spans.push(Span::styled("[", bold));

    if working {
        const FRAMES: [&str; 6] = ["◜", "◠", "◝", "◞", "◡", "◟"];
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let frame = FRAMES[(ms / 120) as usize % FRAMES.len()];
        spans.push(Span::styled(
            format!("{} ", frame),
            Style::default().fg(Color::Blue),
        ));
    }

    spans.push(Span::styled(format!("{}/{}", total, total_items), bold));

    if marked > 0 {
        spans.push(Span::styled(
            format!(" {}◆", marked),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if diff_marked > 0 {
        spans.push(Span::styled(
            format!(" {}◈", diff_marked),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled("] ", bold));

    Line::from(spans)
}

pub(crate) fn build_result_item(
    result: &SearchResult,
    view: &SearchResultsView<'_>,
) -> ListItem<'static> {
    if let SearchItem::GitBranch {
        branch, is_head, ..
    } = &result.item
    {
        let spans = build_git_branch_item_spans(branch, *is_head, &result.indices);
        return build_list_item(
            spans,
            view.marked_items.contains_key(&result.item),
            view.diff_marked_items.contains(&result.item),
        );
    }

    if let SearchItem::GitCommit {
        short_commit,
        subject,
        author,
        date,
        refs,
        ..
    } = &result.item
    {
        let spans =
            build_git_commit_item_spans(short_commit, refs, subject, date, author, &result.indices);
        return build_list_item(
            spans,
            view.marked_items.contains_key(&result.item),
            view.diff_marked_items.contains(&result.item),
        );
    }

    if let SearchItem::GitHistory {
        commit,
        path,
        line,
        text,
    } = &result.item
    {
        let spans = build_git_history_item_spans(commit, path, *line, text, &result.indices);
        return build_list_item(
            spans,
            view.marked_items.contains_key(&result.item),
            view.diff_marked_items.contains(&result.item),
        );
    }

    let original_text = result.item.display_text();
    let original_text = original_text.as_ref();
    let is_marked = view.marked_items.contains_key(&result.item);
    let is_diff_marked = view.diff_marked_items.contains(&result.item);

    // Strip leading "./" — it wastes space and adds no information.
    let (base_text, base_indices): (&str, Cow<[u32]>) = if original_text.starts_with("./") {
        let shifted: Vec<u32> = result
            .indices
            .iter()
            .filter(|&&i| i >= 2)
            .map(|&i| i - 2)
            .collect();
        (&original_text[2..], Cow::Owned(shifted))
    } else {
        (original_text, Cow::Borrowed(&result.indices))
    };

    let (display_text, adjusted_indices) =
        insert_column_if_needed(base_text, &base_indices, result.column);

    let spans = if view.is_content_mode && !view.stdin_mode {
        build_grep_spans(&display_text, adjusted_indices.as_ref())
    } else {
        build_path_spans(&display_text, adjusted_indices.as_ref())
    };

    build_list_item(spans, is_marked, is_diff_marked)
}

fn build_list_item(
    spans: Vec<Span<'static>>,
    is_marked: bool,
    is_diff_marked: bool,
) -> ListItem<'static> {
    if is_marked || is_diff_marked {
        let mut marked_spans = Vec::new();
        if is_marked {
            marked_spans.push(Span::styled("◆ ", Style::default().fg(Color::Green)));
        }
        if is_diff_marked {
            marked_spans.push(Span::styled("◈ ", Style::default().fg(Color::Yellow)));
        }
        marked_spans.extend(spans);
        ListItem::new(Line::from(marked_spans))
    } else {
        ListItem::new(Line::from(spans))
    }
}

struct GrepParts<'a> {
    path: &'a str,
    line: &'a str,
    col: Option<&'a str>,
    content: &'a str,
}

fn parse_grep_display(text: &str) -> Option<GrepParts<'_>> {
    let first_colon = text.find(':')?;
    let path = &text[..first_colon];
    let after_path = &text[first_colon + 1..];

    let second_colon_rel = after_path.find(':')?;
    let line = &after_path[..second_colon_rel];
    if line.is_empty() || !line.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let after_line = &after_path[second_colon_rel + 1..];

    if let Some(third_colon_rel) = after_line.find(':') {
        let maybe_col = &after_line[..third_colon_rel];
        if !maybe_col.is_empty() && maybe_col.chars().all(|c| c.is_ascii_digit()) {
            return Some(GrepParts {
                path,
                line,
                col: Some(maybe_col),
                content: &after_line[third_colon_rel + 1..],
            });
        }
    }

    Some(GrepParts {
        path,
        line,
        col: None,
        content: after_line,
    })
}

const MAX_CONTENT_DISPLAY_CHARS: usize = 500;

fn build_grep_spans(text: &str, indices: &[u32]) -> Vec<Span<'static>> {
    let Some(parts) = parse_grep_display(text) else {
        return build_path_spans(text, indices);
    };

    let sep = Style::default().fg(Color::DarkGray);
    let path = Style::default().fg(Color::Blue);
    let line = Style::default().fg(Color::Yellow);
    let col = Style::default().fg(Color::DarkGray);
    let content_style = Style::default();

    let content_trimmed = parts.content.trim_end_matches(['\n', '\r']);

    let (content_display, was_truncated) =
        truncate_str_chars(content_trimmed, MAX_CONTENT_DISPLAY_CHARS);

    let mut segments: Vec<(&str, Style)> = vec![
        (parts.path, path),
        (":", sep),
        (parts.line, line),
        (":", sep),
    ];
    if let Some(c) = parts.col {
        segments.push((c, col));
        segments.push((":", sep));
    }
    segments.push((content_display, content_style));
    if was_truncated {
        segments.push(("…", Style::default().fg(Color::DarkGray)));
    }

    colored_spans(&segments, indices)
}

pub(crate) fn build_git_history_item_spans(
    commit: &str,
    path: &str,
    line: usize,
    content: &str,
    indices: &[u32],
) -> Vec<Span<'static>> {
    let commit_style = Style::default().fg(Color::Magenta);
    let current_commit_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let path_style = Style::default().fg(Color::Blue);
    let line_style = Style::default().fg(Color::Yellow);
    let sep_style = Style::default().fg(Color::DarkGray);
    let content_style = Style::default();
    let line_string = line.to_string();
    let display_path = path.replace(HISTORY_PATH_SEPARATOR, "/");
    let (content_display, was_truncated) = truncate_str_chars(content, MAX_CONTENT_DISPLAY_CHARS);
    let commit_style = if is_current_commit(commit) {
        current_commit_style
    } else {
        commit_style
    };

    let mut segments: Vec<(&str, Style)> = vec![
        (commit, commit_style),
        (": ", sep_style),
        (&display_path, path_style),
        (":", sep_style),
        (&line_string, line_style),
        (":", sep_style),
        (content_display, content_style),
    ];
    if was_truncated {
        segments.push(("…", sep_style));
    }

    colored_spans(&segments, indices)
}

fn build_git_branch_item_spans(branch: &str, is_head: bool, indices: &[u32]) -> Vec<Span<'static>> {
    let style = if is_head {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Blue)
    };
    colored_spans(&[(branch, style)], indices)
}

fn build_git_commit_item_spans(
    short_commit: &str,
    refs: &str,
    subject: &str,
    date: &str,
    author: &str,
    indices: &[u32],
) -> Vec<Span<'static>> {
    let short_hash = truncate_commit_hash(short_commit);
    let refs = refs.trim();
    let hash_style = Style::default().fg(Color::DarkGray);
    let ref_style = Style::default().fg(Color::Green);
    let subject_style = Style::default();
    let meta_style = Style::default().fg(Color::DarkGray);

    let mut segments: Vec<(&str, Style)> = vec![(&short_hash, hash_style), (" - ", meta_style)];
    if !refs.is_empty() {
        segments.push(("(", meta_style));
        segments.push((refs, ref_style));
        segments.push((") ", meta_style));
    }
    segments.push((subject, subject_style));
    segments.push((" (", meta_style));
    segments.push((date, meta_style));
    segments.push((") <", meta_style));
    segments.push((author, meta_style));
    segments.push((">", meta_style));
    colored_spans(&segments, indices)
}

fn truncate_commit_hash(hash: &str) -> String {
    let short: String = hash.chars().take(5).collect();
    format!("[{short}]")
}

fn build_path_spans(text: &str, indices: &[u32]) -> Vec<Span<'static>> {
    let dir_style = Style::default().fg(Color::Gray);
    let file_style = Style::default().fg(Color::Blue);

    let last_sep = text.rfind('/').or_else(|| text.rfind('\\'));
    let segments: Vec<(&str, Style)> = if let Some(pos) = last_sep {
        vec![
            (&text[..pos + 1], dir_style),
            (&text[pos + 1..], file_style),
        ]
    } else {
        vec![(text, file_style)]
    };

    colored_spans(&segments, indices)
}

fn colored_spans(segments: &[(&str, Style)], match_chars: &[u32]) -> Vec<Span<'static>> {
    let highlight = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut global_char = 0usize;
    let mut match_idx = 0usize;

    let is_match = |char_idx: usize, match_idx: &mut usize| -> bool {
        while *match_idx < match_chars.len() && (match_chars[*match_idx] as usize) < char_idx {
            *match_idx += 1;
        }
        *match_idx < match_chars.len() && match_chars[*match_idx] as usize == char_idx
    };

    for &(seg_text, base_style) in segments {
        if seg_text.is_empty() {
            continue;
        }

        let chars: Vec<(usize, char)> = seg_text.char_indices().collect();
        let mut span_start_byte = 0usize;
        let mut cur_style = if is_match(global_char, &mut match_idx) {
            highlight
        } else {
            base_style
        };

        for (local_idx, &(byte_pos, _ch)) in chars.iter().enumerate() {
            let eff = if is_match(global_char + local_idx, &mut match_idx) {
                highlight
            } else {
                base_style
            };

            if eff != cur_style {
                let text = seg_text[span_start_byte..byte_pos].to_string();
                if !text.is_empty() {
                    spans.push(Span::styled(text, cur_style));
                }
                span_start_byte = byte_pos;
                cur_style = eff;
            }
        }

        let tail = seg_text[span_start_byte..].to_string();
        if !tail.is_empty() {
            spans.push(Span::styled(tail, cur_style));
        }

        global_char += chars.len();
    }

    spans
}

fn insert_column_if_needed<'a>(
    original_text: &str,
    indices: &'a [u32],
    column: Option<usize>,
) -> (String, Cow<'a, [u32]>) {
    let Some(col) = column else {
        return (original_text.to_string(), Cow::Borrowed(indices));
    };

    let Some(insert_pos) = grep_content_prefix_end(original_text) else {
        return (original_text.to_string(), Cow::Borrowed(indices));
    };

    let (prefix, suffix) = original_text.split_at(insert_pos);
    let col_str = format!("{}:", col);
    let new_text = format!("{}{}{}", prefix, col_str, suffix);

    let insert_char_pos = original_text[..insert_pos].chars().count();
    let shift = col_str.chars().count() as u32;

    let new_indices = indices
        .iter()
        .map(|&idx| {
            if idx as usize >= insert_char_pos {
                idx + shift
            } else {
                idx
            }
        })
        .collect();

    (new_text, Cow::Owned(new_indices))
}

fn grep_content_prefix_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut first_colon = None;

    for (i, &b) in bytes.iter().enumerate() {
        if b != b':' {
            continue;
        }

        if let Some(start) = first_colon {
            if i > start + 1 {
                let potential_num = &text[start + 1..i];
                if potential_num.chars().all(|c| c.is_ascii_digit()) {
                    return Some(i + 1);
                }
            }
            first_colon = Some(i);
        } else {
            first_colon = Some(i);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_git_history_hash_uses_distinct_style() {
        let spans = build_git_history_item_spans("HEAD", "Architecture.md", 12, "hello world", &[]);
        assert_eq!(spans[0].content, "HEAD");
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn git_branch_results_show_only_branch_name() {
        let spans = build_git_branch_item_spans("feature/test", false, &[]);
        let rendered = spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(rendered, "feature/test");
    }

    #[test]
    fn git_commit_results_use_short_bracketed_hash_and_metadata() {
        let spans = build_git_commit_item_spans(
            "abcdef1234",
            "main, tag: v1.0",
            "improve preview rendering",
            "2 days ago",
            "jpcrs",
            &[],
        );
        let rendered = spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(
            rendered,
            "[abcde] - (main, tag: v1.0) improve preview rendering (2 days ago) <jpcrs>"
        );
    }

    #[test]
    fn visible_result_range_keeps_selected_row_in_view() {
        let mut state = ListState::default().with_selected(Some(15)).with_offset(0);

        let (start, end, selected) = visible_result_range(&mut state, 100, 8);

        assert_eq!((start, end, selected), (8, 16, Some(7)));
        assert_eq!(state.offset(), 8);
    }

    #[test]
    fn visible_result_range_clamps_offset_when_results_shrink() {
        let mut state = ListState::default().with_selected(Some(2)).with_offset(50);

        let (start, end, selected) = visible_result_range(&mut state, 5, 10);

        assert_eq!((start, end, selected), (0, 5, Some(2)));
        assert_eq!(state.offset(), 0);
    }
}
