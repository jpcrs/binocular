use crate::search::types::SearchItem;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreviewSource {
    SearchItem(SearchItem),
    Diff {
        left: String,
        right: String,
    },
    GitHistory {
        commit: String,
        path: String,
        line: usize,
    },
    GitBranch {
        branch: String,
    },
    GitCommit {
        commit: String,
    },
    LogStream(String),
}

impl PreviewSource {
    pub fn title(&self) -> Cow<'_, str> {
        match self {
            Self::SearchItem(item) => match item {
                SearchItem::Path(path) => Cow::Borrowed(path.as_str()),
                SearchItem::Grep { path, .. } => Cow::Borrowed(path.as_str()),
                SearchItem::GitHistory {
                    commit, path, line, ..
                } => Cow::Owned(format!("{commit}: {path}:{line}")),
                SearchItem::GitBranch { branch, .. } => Cow::Owned(format!("branch: {branch}")),
                SearchItem::GitCommit { commit, .. } => Cow::Owned(format!("commit: {commit}")),
                SearchItem::Stdin(_) => Cow::Borrowed("<stdin>"),
                SearchItem::Message(_) => Cow::Borrowed("Message"),
            },
            Self::Diff { left, right } => Cow::Owned(format!("diff: {left} <> {right}")),
            Self::GitHistory { commit, path, line } => {
                Cow::Owned(format!("{commit}: {path}:{line}"))
            }
            Self::GitBranch { branch } => Cow::Owned(format!("branch: {branch}")),
            Self::GitCommit { commit } => Cow::Owned(format!("commit: {commit}")),
            Self::LogStream(path) => Cow::Borrowed(path.as_str()),
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::SearchItem(item) => item.preview_path(),
            Self::Diff { .. } => None,
            Self::GitHistory { path, .. } => Some(path.as_str()),
            Self::GitBranch { .. } | Self::GitCommit { .. } => None,
            Self::LogStream(path) => Some(path.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreviewRequest {
    Path {
        source: PreviewSource,
        path: String,
    },
    Diff {
        source: PreviewSource,
        left: String,
        right: String,
    },
    GitHistory {
        source: PreviewSource,
        repo_root: String,
        commit: String,
        path: String,
        line: usize,
    },
    GitBranch {
        source: PreviewSource,
        repo_root: String,
        branch: String,
    },
    GitCommit {
        source: PreviewSource,
        repo_root: String,
        commit: String,
    },
    Grep {
        source: PreviewSource,
        path: String,
        line: usize,
        text: String,
    },
    StdinOrCommand {
        source: PreviewSource,
        item: String,
    },
    StructuredLog {
        source: PreviewSource,
        path: String,
    },
}

impl PreviewRequest {
    pub fn from_source(source: PreviewSource, has_preview_command: bool) -> Self {
        match source {
            PreviewSource::LogStream(path) => Self::StructuredLog {
                source: PreviewSource::LogStream(path.clone()),
                path,
            },
            PreviewSource::Diff { left, right } => Self::Diff {
                source: PreviewSource::Diff {
                    left: left.clone(),
                    right: right.clone(),
                },
                left,
                right,
            },
            PreviewSource::GitHistory { commit, path, line } => Self::GitHistory {
                source: PreviewSource::GitHistory {
                    commit: commit.clone(),
                    path: path.clone(),
                    line,
                },
                repo_root: String::new(),
                commit,
                path,
                line,
            },
            PreviewSource::GitBranch { branch } => Self::GitBranch {
                source: PreviewSource::GitBranch {
                    branch: branch.clone(),
                },
                repo_root: String::new(),
                branch,
            },
            PreviewSource::GitCommit { commit } => Self::GitCommit {
                source: PreviewSource::GitCommit {
                    commit: commit.clone(),
                },
                repo_root: String::new(),
                commit,
            },
            PreviewSource::SearchItem(item) => {
                if has_preview_command {
                    return Self::StdinOrCommand {
                        source: PreviewSource::SearchItem(item.clone()),
                        item: item.display_text().into_owned(),
                    };
                }

                match item {
                    SearchItem::Path(path) => Self::Path {
                        source: PreviewSource::SearchItem(SearchItem::Path(path.clone())),
                        path,
                    },
                    SearchItem::Grep { path, line, text } => Self::Grep {
                        source: PreviewSource::SearchItem(SearchItem::Grep {
                            path: path.clone(),
                            line,
                            text: text.clone(),
                        }),
                        path,
                        line,
                        text,
                    },
                    SearchItem::GitHistory {
                        commit, path, line, ..
                    } => Self::GitHistory {
                        source: PreviewSource::GitHistory {
                            commit: commit.clone(),
                            path: path.clone(),
                            line,
                        },
                        repo_root: String::new(),
                        commit,
                        path,
                        line,
                    },
                    SearchItem::GitBranch { branch, .. } => Self::GitBranch {
                        source: PreviewSource::GitBranch {
                            branch: branch.clone(),
                        },
                        repo_root: String::new(),
                        branch,
                    },
                    SearchItem::GitCommit { commit, .. } => Self::GitCommit {
                        source: PreviewSource::GitCommit {
                            commit: commit.clone(),
                        },
                        repo_root: String::new(),
                        commit,
                    },
                    SearchItem::Stdin(text) => Self::StdinOrCommand {
                        source: PreviewSource::SearchItem(SearchItem::Stdin(text.clone())),
                        item: text,
                    },
                    SearchItem::Message(text) => Self::StdinOrCommand {
                        source: PreviewSource::SearchItem(SearchItem::Message(text.clone())),
                        item: text,
                    },
                }
            }
        }
    }

    pub fn source(&self) -> &PreviewSource {
        match self {
            Self::Path { source, .. }
            | Self::Diff { source, .. }
            | Self::GitHistory { source, .. }
            | Self::GitBranch { source, .. }
            | Self::GitCommit { source, .. }
            | Self::Grep { source, .. }
            | Self::StdinOrCommand { source, .. }
            | Self::StructuredLog { source, .. } => source,
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::Path { path, .. }
            | Self::GitHistory { path, .. }
            | Self::Grep { path, .. }
            | Self::StructuredLog { path, .. } => Some(path.as_str()),
            Self::Diff { .. }
            | Self::GitBranch { .. }
            | Self::GitCommit { .. }
            | Self::StdinOrCommand { .. } => None,
        }
    }
}
