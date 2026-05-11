use crate::preview::request::command::execute_preview_command;
use crate::preview::request::diff::build_diff_preview;
use crate::preview::request::git::{
    branch::build_git_branch_preview, commit::build_git_commit_preview,
    history::build_history_preview,
};
use crate::preview::{build_path_preview, PreviewContent, PreviewRequest};
use ratatui::text::Text;
use ratatui_image::picker::Picker;

pub(crate) enum PreviewExecution {
    Completed(PreviewRequest, PreviewContent),
    Superseded(PreviewRequest),
}
pub(crate) struct PreviewExecutor {
    picker: Picker,
    preview_command: Option<String>,
    delimiter: String,
    log_max_entries: usize,
}

impl PreviewExecutor {
    pub(crate) fn new(
        picker: Picker,
        preview_command: Option<String>,
        delimiter: String,
        log_max_entries: usize,
    ) -> Self {
        Self {
            picker,
            preview_command,
            delimiter,
            log_max_entries,
        }
    }

    pub(crate) fn execute<F>(
        &self,
        request: PreviewRequest,
        mut poll_replacement: F,
    ) -> PreviewExecution
    where
        F: FnMut() -> Option<PreviewRequest>,
    {
        match request {
            PreviewRequest::Path { source, path } => self.execute_builtin_preview(
                PreviewRequest::Path {
                    source,
                    path: path.clone(),
                },
                &path,
            ),
            PreviewRequest::Diff {
                source,
                left,
                right,
            } => {
                let preview = build_diff_preview(&left, &right);
                PreviewExecution::Completed(
                    PreviewRequest::Diff {
                        source,
                        left,
                        right,
                    },
                    preview,
                )
            }
            PreviewRequest::GitHistory {
                source,
                repo_root,
                commit,
                path,
                line,
            } => {
                let preview = build_history_preview(&repo_root, &commit, &path);
                PreviewExecution::Completed(
                    PreviewRequest::GitHistory {
                        source,
                        repo_root,
                        commit,
                        path,
                        line,
                    },
                    preview,
                )
            }
            PreviewRequest::GitBranch {
                source,
                repo_root,
                branch,
            } => {
                let preview = build_git_branch_preview(&repo_root, &branch);
                PreviewExecution::Completed(
                    PreviewRequest::GitBranch {
                        source,
                        repo_root,
                        branch,
                    },
                    preview,
                )
            }
            PreviewRequest::GitCommit {
                source,
                repo_root,
                commit,
            } => {
                let preview = build_git_commit_preview(&repo_root, &commit);
                PreviewExecution::Completed(
                    PreviewRequest::GitCommit {
                        source,
                        repo_root,
                        commit,
                    },
                    preview,
                )
            }
            PreviewRequest::Grep {
                source,
                path,
                line,
                text,
            } => self.execute_builtin_preview(
                PreviewRequest::Grep {
                    source,
                    path: path.clone(),
                    line,
                    text,
                },
                &path,
            ),
            PreviewRequest::StructuredLog { source, path } => self.execute_builtin_preview(
                PreviewRequest::StructuredLog {
                    source,
                    path: path.clone(),
                },
                &path,
            ),
            PreviewRequest::StdinOrCommand { source, item } => {
                let request = PreviewRequest::StdinOrCommand {
                    source,
                    item: item.clone(),
                };
                if let Some(command) = self.preview_command.as_deref() {
                    execute_preview_command(
                        request,
                        &item,
                        command,
                        &self.delimiter,
                        &mut poll_replacement,
                    )
                } else {
                    PreviewExecution::Completed(request, PreviewContent::PlainText(Text::default()))
                }
            }
        }
    }

    fn execute_builtin_preview(&self, request: PreviewRequest, path: &str) -> PreviewExecution {
        let preview = build_path_preview(path, &self.picker, self.log_max_entries);
        PreviewExecution::Completed(request, preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::request::git::ansi::parse_ansi_text;
    use crate::preview::PreviewSource;
    use crate::search::types::SearchItem;
    use ratatui::style::Color;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn command_request(item: &str) -> PreviewRequest {
        PreviewRequest::StdinOrCommand {
            source: PreviewSource::SearchItem(SearchItem::stdin(item)),
            item: item.to_string(),
        }
    }

    #[test]
    fn preview_command_replacement_returns_latest_request() {
        let executor = PreviewExecutor::new(
            Picker::halfblocks(),
            Some("sh -c 'sleep 1'".to_string()),
            ":".to_string(),
            100_000,
        );
        let replacement = command_request("replacement");
        let polls = AtomicUsize::new(0);

        let outcome = executor.execute(command_request("initial"), || {
            if polls.fetch_add(1, Ordering::Relaxed) == 0 {
                Some(replacement.clone())
            } else {
                None
            }
        });

        match outcome {
            PreviewExecution::Superseded(request) => assert_eq!(request, replacement),
            PreviewExecution::Completed(_, _) => panic!("expected replacement"),
        }
    }

    #[test]
    fn preview_command_timeout_surfaces_message() {
        let executor = PreviewExecutor::new(
            Picker::halfblocks(),
            Some("sh -c 'sleep 3'".to_string()),
            ":".to_string(),
            100_000,
        );

        let outcome = executor.execute(command_request("initial"), || None);

        match outcome {
            PreviewExecution::Completed(_, PreviewContent::PlainText(text)) => {
                let rendered = text
                    .lines
                    .iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(rendered.contains("timed out"));
            }
            PreviewExecution::Completed(_, _) => panic!("expected plain text timeout"),
            PreviewExecution::Superseded(_) => panic!("expected timeout"),
        }
    }

    #[test]
    fn ansi_empty_reset_code_resets_style_before_next_line() {
        let text = parse_ansi_text("\u{1b}[31m--\u{1b}[m\nfile.rs\n".to_string());

        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].spans.len(), 1);
        assert_eq!(text.lines[0].spans[0].content, "--");
        assert_eq!(text.lines[0].spans[0].style.fg, Some(Color::Red));

        assert_eq!(text.lines[1].spans.len(), 1);
        assert_eq!(text.lines[1].spans[0].content, "file.rs");
        assert_ne!(text.lines[1].spans[0].style.fg, Some(Color::Red));
    }
}
