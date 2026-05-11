use super::SelectionOutput;
use crate::cli::args::OutputFormat;
use crate::search::sources::git::HISTORY_PATH_SEPARATOR;
use crate::search::types::SearchItem;
use std::path::Path;

impl SelectionOutput {
    pub fn render(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Plain => self.render_plain(),
            OutputFormat::Jsonl => self.render_jsonl(),
        }
    }

    fn render_plain(&self) -> String {
        match self {
            Self::Item { item, column } => format_item_output(item, *column, true),
            Self::PreviewLocation { path, row, column } => format!("{path}:{row}:{column}"),
        }
    }

    fn render_jsonl(&self) -> String {
        match self {
            Self::Item {
                item: SearchItem::Stdin(text),
                ..
            } => serde_json::json!({
                "kind": "stdin",
                "text": text,
            })
            .to_string(),
            Self::Item {
                item: SearchItem::Path(path),
                ..
            } => serde_json::json!({
                "kind": "path",
                "path": canonicalize_or_clone(path),
            })
            .to_string(),
            Self::Item {
                item: SearchItem::Grep { path, line, .. },
                column,
            } => {
                let mut value = serde_json::json!({
                    "kind": "grep",
                    "path": canonicalize_or_clone(path),
                    "line": line,
                });
                if let Some(column) = column {
                    value["column"] = serde_json::json!(column);
                }
                value.to_string()
            }
            Self::Item {
                item:
                    SearchItem::GitHistory {
                        commit, path, line, ..
                    },
                column,
            } => {
                let mut value = serde_json::json!({
                    "kind": "git_history",
                    "commit": commit,
                    "path": path,
                    "line": line,
                });
                if let Some(column) = column {
                    value["column"] = serde_json::json!(column);
                }
                value.to_string()
            }
            Self::Item {
                item: SearchItem::GitBranch { branch, .. },
                ..
            } => serde_json::json!({
                "kind": "git_branch",
                "branch": branch,
            })
            .to_string(),
            Self::Item {
                item: SearchItem::GitCommit { commit, .. },
                ..
            } => serde_json::json!({
                "kind": "git_commit",
                "commit": commit,
            })
            .to_string(),
            Self::Item {
                item: SearchItem::Message(text),
                ..
            } => serde_json::json!({
                "kind": "message",
                "text": text,
            })
            .to_string(),
            Self::PreviewLocation { path, row, column } => serde_json::json!({
                "kind": "preview_location",
                "path": canonicalize_or_clone(path),
                "line": row,
                "column": column,
            })
            .to_string(),
        }
    }
}

pub fn format_item_output(item: &SearchItem, column: Option<usize>, canonicalize: bool) -> String {
    match item {
        SearchItem::Stdin(text) | SearchItem::Message(text) => text.clone(),
        SearchItem::Path(path) => {
            if canonicalize {
                Path::new(path)
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.clone())
            } else {
                path.clone()
            }
        }
        SearchItem::Grep { path, line, .. } => {
            let abs_path = if canonicalize {
                Path::new(path)
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.clone())
            } else {
                path.clone()
            };

            if let Some(col) = column {
                format!("{}:{}:{}", abs_path, line, col)
            } else {
                format!("{}:{}", abs_path, line)
            }
        }
        SearchItem::GitHistory {
            commit, path, line, ..
        } => {
            let display_path = path.replace(HISTORY_PATH_SEPARATOR, "/");
            format!("{}:{}:{}", commit, display_path, line)
        }
        SearchItem::GitBranch { branch, .. } => branch.clone(),
        SearchItem::GitCommit { commit, .. } => commit.clone(),
    }
}

pub fn render_selection_outputs(
    outputs: &[SelectionOutput],
    format: OutputFormat,
) -> Option<String> {
    if outputs.is_empty() {
        return None;
    }

    Some(
        outputs
            .iter()
            .map(|output| output.render(format))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn canonicalize_or_clone(path: &str) -> String {
    Path::new(path)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_item_output_preserves_windows_style_paths() {
        let item = SearchItem::path(r"C:\work\project:file.rs");
        assert_eq!(
            format_item_output(&item, None, false),
            r"C:\work\project:file.rs"
        );
    }

    #[test]
    fn format_item_output_handles_grep_column_edge_cases() {
        let item = SearchItem::grep(r"C:\work\main.rs", 42, "let value = 1;");
        assert_eq!(
            format_item_output(&item, Some(7), false),
            r"C:\work\main.rs:42:7"
        );
        assert_eq!(
            format_item_output(&item, None, false),
            r"C:\work\main.rs:42"
        );
    }

    #[test]
    fn format_item_output_renders_git_history_item() {
        let item = SearchItem::history_line("abc123", "Architecture.md", 42, "text");
        assert_eq!(
            format_item_output(&item, None, false),
            "abc123:Architecture.md:42"
        );
    }

    #[test]
    fn jsonl_path_output_is_machine_readable() {
        let rendered = SelectionOutput::Item {
            item: SearchItem::path(r"C:\work\project:file.rs"),
            column: None,
        }
        .render(OutputFormat::Jsonl);

        assert_eq!(
            rendered,
            serde_json::json!({
                "kind": "path",
                "path": r"C:\work\project:file.rs",
            })
            .to_string()
        );
    }

    #[test]
    fn jsonl_grep_output_keeps_optional_column() {
        let rendered = SelectionOutput::Item {
            item: SearchItem::grep(r"C:\work\main.rs", 42, "let value = 1;"),
            column: Some(7),
        }
        .render(OutputFormat::Jsonl);

        assert_eq!(
            rendered,
            serde_json::json!({
                "kind": "grep",
                "path": r"C:\work\main.rs",
                "line": 42,
                "column": 7,
            })
            .to_string()
        );
    }

    #[test]
    fn jsonl_preview_output_uses_line_and_column_fields() {
        let rendered = SelectionOutput::PreviewLocation {
            path: r"C:\work\main.rs".to_string(),
            row: 24,
            column: 4,
        }
        .render(OutputFormat::Jsonl);

        assert_eq!(
            rendered,
            serde_json::json!({
                "kind": "preview_location",
                "path": r"C:\work\main.rs",
                "line": 24,
                "column": 4,
            })
            .to_string()
        );
    }

    #[test]
    fn render_selection_outputs_joins_multiple_records() {
        let rendered = render_selection_outputs(
            &[
                SelectionOutput::Item {
                    item: SearchItem::path("first.txt"),
                    column: None,
                },
                SelectionOutput::Item {
                    item: SearchItem::path("second.txt"),
                    column: None,
                },
            ],
            OutputFormat::Jsonl,
        )
        .expect("joined output");

        let lines: Vec<_> = rendered.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"kind\":\"path\""));
        assert!(lines[1].contains("\"kind\":\"path\""));
    }
}
