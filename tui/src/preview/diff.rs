use crate::preview::{DiffPreview, PreviewContent};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use similar::{ChangeTag, DiffOp, TextDiff};
use std::fs;
use std::path::Path;

pub fn build_diff_preview(left_path: &str, right_path: &str) -> PreviewContent {
    let left = match read_text_file(left_path) {
        Ok(content) => content,
        Err(message) => return PreviewContent::PlainText(Text::from(message)),
    };
    let right = match read_text_file(right_path) {
        Ok(content) => content,
        Err(message) => return PreviewContent::PlainText(Text::from(message)),
    };

    let diff = TextDiff::from_lines(&left, &right);
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("--- ", Style::default().fg(Color::Red)),
        Span::raw(left_path.to_string()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("+++ ", Style::default().fg(Color::Green)),
        Span::raw(right_path.to_string()),
    ]));
    lines.push(Line::default());

    for group in diff.grouped_ops(3) {
        lines.push(build_hunk_header(&group));
        for op in group {
            render_op_lines(&diff, &op, &mut lines);
        }
    }

    PreviewContent::Diff(DiffPreview {
        text: Text::from(lines),
    })
}

/// Maximum file size to load for diff preview (50 MiB).
const MAX_DIFF_FILE_SIZE: u64 = 50 * 1024 * 1024;

fn read_text_file(path: &str) -> Result<String, String> {
    let path_obj = Path::new(path);

    // SECURITY: reject files that are too large to prevent OOM.
    let metadata = fs::metadata(path_obj)
        .map_err(|err| format!("Failed to read {}: {}", path_obj.display(), err))?;
    if metadata.len() > MAX_DIFF_FILE_SIZE {
        return Err(format!(
            "{} is too large to diff ({} > {} MiB)",
            path_obj.display(),
            metadata.len() / (1024 * 1024),
            MAX_DIFF_FILE_SIZE / (1024 * 1024)
        ));
    }

    let bytes = fs::read(path_obj)
        .map_err(|err| format!("Failed to read {}: {}", path_obj.display(), err))?;

    if bytes.contains(&0)
        || crate::text::proportion_of_printable_ascii_characters(&bytes)
            < crate::text::PRINTABLE_ASCII_THRESHOLD
    {
        return Err(format!("{} is not a text file", path_obj.display()));
    }

    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }

    if let Some(decoded) = crate::preview::encoding::try_decode_utf16(&bytes) {
        return Ok(decoded);
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn build_hunk_header(group: &[DiffOp]) -> Line<'static> {
    let old_start = group
        .iter()
        .map(|op| op.old_range().start)
        .min()
        .unwrap_or(0)
        + 1;
    let old_end = group.iter().map(|op| op.old_range().end).max().unwrap_or(0);
    let new_start = group
        .iter()
        .map(|op| op.new_range().start)
        .min()
        .unwrap_or(0)
        + 1;
    let new_end = group.iter().map(|op| op.new_range().end).max().unwrap_or(0);
    let old_len = old_end.saturating_sub(old_start.saturating_sub(1));
    let new_len = new_end.saturating_sub(new_start.saturating_sub(1));

    Line::from(vec![Span::styled(
        format!(
            "@@ -{},{} +{},{} @@",
            old_start, old_len, new_start, new_len
        ),
        Style::default().fg(Color::Cyan),
    )])
}

fn render_op_lines<'a>(
    diff: &'a TextDiff<'a, 'a, str>,
    op: &DiffOp,
    lines: &mut Vec<Line<'static>>,
) {
    for change in diff.iter_inline_changes(op) {
        let (sign, base_style, emphasize_style) = match change.tag() {
            ChangeTag::Delete => (
                "-",
                Style::default().fg(Color::Red),
                Style::default().fg(Color::Black).bg(Color::Red),
            ),
            ChangeTag::Insert => (
                "+",
                Style::default().fg(Color::Green),
                Style::default().fg(Color::Black).bg(Color::Green),
            ),
            ChangeTag::Equal => (
                " ",
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let old_line = format_line_number(change.old_index());
        let new_line = format_line_number(change.new_index());
        let gutter_style = Style::default().fg(Color::DarkGray);
        let mut spans = vec![
            Span::styled(format!("{} {} | ", old_line, new_line), gutter_style),
            Span::styled(sign, base_style),
        ];
        for (emphasized, value) in change.iter_strings_lossy() {
            let style = if emphasized {
                emphasize_style
            } else {
                base_style
            };
            for segment in value.split_inclusive('\n') {
                let segment = segment.trim_end_matches('\n');
                if !segment.is_empty() {
                    spans.push(Span::styled(segment.to_string(), style));
                }
                if value.ends_with('\n') {
                    lines.push(Line::from(std::mem::take(&mut spans)));
                    spans.push(Span::styled(
                        format!("{} {} | ", old_line, new_line),
                        gutter_style,
                    ));
                    spans.push(Span::styled(sign, base_style));
                }
            }
        }

        if spans.len() > 1 {
            lines.push(Line::from(spans));
        }
    }
}

fn format_line_number(index: Option<usize>) -> String {
    index
        .map(|value| format!("{:>4}", value + 1))
        .unwrap_or_else(|| "    ".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("binocular-diff-{name}-{nanos}.txt"))
    }

    #[test]
    fn diff_preview_renders_changed_lines() {
        let left = unique_temp_path("left");
        let right = unique_temp_path("right");
        std::fs::write(&left, "alpha\nbeta\n").unwrap();
        std::fs::write(&right, "alpha\ngamma\n").unwrap();

        let preview = build_diff_preview(&left.display().to_string(), &right.display().to_string());
        let PreviewContent::Diff(diff) = preview else {
            panic!("expected diff preview");
        };

        let rendered = diff
            .text
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("--- "));
        assert!(rendered.contains("+++ "));
        assert!(rendered.contains("@@ -1,2 +1,2 @@"));
        assert!(rendered.contains("   2      | -beta") || rendered.contains("   2    2 | -beta"));
        assert!(rendered.contains("      2 | +gamma") || rendered.contains("   2    2 | +gamma"));

        let _ = std::fs::remove_file(left);
        let _ = std::fs::remove_file(right);
    }
}
