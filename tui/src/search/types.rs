use crate::cli::args::Args;
use crate::search::sources::git::HISTORY_PATH_SEPARATOR;
use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Path,
    Files,
    Grep,
    Dirs,
    GitHistory,
    GitBranches,
    GitCommits,
}

impl SearchMode {
    pub fn from_args(args: &Args) -> Self {
        if args.git_history.is_some() {
            Self::GitHistory
        } else if args.git_branches {
            Self::GitBranches
        } else if args.git_commits {
            Self::GitCommits
        } else if args.content {
            Self::Grep
        } else if args.dir_only {
            Self::Dirs
        } else if args.file_name {
            Self::Files
        } else {
            Self::Path
        }
    }

    pub fn is_content(self) -> bool {
        matches!(self, Self::Grep | Self::GitHistory)
    }

    pub fn is_dir_only(self) -> bool {
        matches!(self, Self::Dirs)
    }

    pub fn is_file_name_only(self) -> bool {
        matches!(self, Self::Files)
    }

    pub fn display_name(self, stdin: bool) -> &'static str {
        if stdin {
            "Stdin"
        } else {
            match self {
                Self::Path => "Path",
                Self::Files => "Files",
                Self::Grep => "Grep",
                Self::Dirs => "Dirs",
                Self::GitHistory => "History",
                Self::GitBranches => "Branches",
                Self::GitCommits => "Commits",
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherMode {
    Fuzzy,
    Exact,
}

impl MatcherMode {
    pub fn from_args(args: &Args) -> Self {
        if args.exact {
            Self::Exact
        } else {
            Self::Fuzzy
        }
    }

    pub fn is_exact(self) -> bool {
        matches!(self, Self::Exact)
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Fuzzy => Self::Exact,
            Self::Exact => Self::Fuzzy,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Fuzzy => "Fuzzy",
            Self::Exact => "Exact",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchSettings {
    pub mode: SearchMode,
    pub matcher: MatcherMode,
}

impl SearchSettings {
    pub fn from_args(args: &Args) -> Self {
        Self {
            mode: SearchMode::from_args(args),
            matcher: MatcherMode::from_args(args),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchConfig {
    pub query: Option<String>,
    pub locations: Vec<PathBuf>,
    pub search_pdf: bool,
    pub no_hidden: bool,
    pub no_git_ignore: bool,
    pub no_ignore: bool,
    pub no_default_ignore_dirs: bool,
    pub git_search_scope: Option<crate::search::sources::git::GitSearchScope>,
    pub settings: SearchSettings,
}

impl SearchConfig {
    pub fn from_args(args: &Args) -> Self {
        Self {
            query: args.query.clone(),
            locations: args.location.clone(),
            search_pdf: args.search_pdf,
            no_hidden: args.no_hidden,
            no_git_ignore: args.no_git_ignore,
            no_ignore: args.no_ignore,
            no_default_ignore_dirs: args.no_default_ignore_dirs,
            git_search_scope: args.git_search_scope.clone(),
            settings: SearchSettings::from_args(args),
        }
    }

    pub fn with_settings(&self, settings: SearchSettings) -> Self {
        let mut config = self.clone();
        config.settings = settings;
        config
    }

    pub fn with_git_search_scope(
        &self,
        git_search_scope: Option<crate::search::sources::git::GitSearchScope>,
    ) -> Self {
        let mut config = self.clone();
        config.git_search_scope = git_search_scope;
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SearchItem {
    Path(String),
    Grep {
        path: String,
        line: usize,
        text: String,
    },
    GitHistory {
        commit: String,
        path: String,
        line: usize,
        text: String,
    },
    GitBranch {
        branch: String,
        commit: String,
        subject: String,
        is_head: bool,
        relative_time: String,
    },
    GitCommit {
        commit: String,
        short_commit: String,
        subject: String,
        author: String,
        date: String,
        refs: String,
    },
    Stdin(String),
    Message(String),
}

impl SearchItem {
    pub fn path(path: impl Into<String>) -> Self {
        Self::Path(path.into())
    }

    pub fn grep(path: impl Into<String>, line: usize, text: impl Into<String>) -> Self {
        Self::Grep {
            path: path.into(),
            line,
            text: text.into(),
        }
    }

    pub fn stdin(text: impl Into<String>) -> Self {
        Self::Stdin(text.into())
    }

    pub fn history_line(
        commit: impl Into<String>,
        path: impl Into<String>,
        line: usize,
        text: impl Into<String>,
    ) -> Self {
        Self::GitHistory {
            commit: commit.into(),
            path: path.into(),
            line,
            text: text.into(),
        }
    }

    pub fn history_error(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn message(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }

    pub fn git_branch(
        branch: impl Into<String>,
        commit: impl Into<String>,
        subject: impl Into<String>,
        is_head: bool,
        relative_time: impl Into<String>,
    ) -> Self {
        Self::GitBranch {
            branch: branch.into(),
            commit: commit.into(),
            subject: subject.into(),
            is_head,
            relative_time: relative_time.into(),
        }
    }

    pub fn git_commit(
        commit: impl Into<String>,
        short_commit: impl Into<String>,
        subject: impl Into<String>,
        author: impl Into<String>,
        date: impl Into<String>,
        refs: impl Into<String>,
    ) -> Self {
        Self::GitCommit {
            commit: commit.into(),
            short_commit: short_commit.into(),
            subject: subject.into(),
            author: author.into(),
            date: date.into(),
            refs: refs.into(),
        }
    }

    pub fn match_text(&self, use_filename_only: bool) -> Cow<'_, str> {
        match self {
            Self::Path(path) => {
                if use_filename_only {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(Cow::Borrowed)
                        .unwrap_or_else(|| Cow::Borrowed(path.as_str()))
                } else {
                    Cow::Borrowed(path.as_str())
                }
            }
            Self::Grep { path, line, text } => {
                if use_filename_only {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(Cow::Borrowed)
                        .unwrap_or_else(|| Cow::Borrowed(path.as_str()))
                } else {
                    Cow::Owned(format!("{path}:{line}:{text}"))
                }
            }
            Self::GitHistory {
                commit,
                path,
                line,
                text,
            } => {
                if use_filename_only {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(Cow::Borrowed)
                        .unwrap_or_else(|| Cow::Borrowed(path.as_str()))
                } else {
                    Cow::Owned(format!("{commit} {path}:{line}:{text}"))
                }
            }
            Self::GitBranch {
                branch,
                commit,
                subject,
                is_head,
                relative_time,
            } => {
                let head = if *is_head { "HEAD " } else { "" };
                Cow::Owned(format!("{head}{branch} {commit} {subject} {relative_time}"))
            }
            Self::GitCommit {
                short_commit,
                subject,
                author,
                date,
                refs,
                ..
            } => Cow::Owned(format!("{short_commit} {refs} {subject} {date} {author}")),
            Self::Stdin(text) => Cow::Borrowed(text.as_str()),
            Self::Message(text) => Cow::Borrowed(text.as_str()),
        }
    }

    pub fn display_text(&self) -> Cow<'_, str> {
        match self {
            Self::Path(path) => Cow::Borrowed(path.as_str()),
            Self::Grep { path, line, text } => Cow::Owned(format!("{path}:{line}:{text}")),
            Self::GitHistory {
                commit,
                path,
                line,
                text,
            } => Cow::Owned(format!("{commit}: {path}:{line}:{text}")),
            Self::GitBranch { branch, .. } => Cow::Borrowed(branch.as_str()),
            Self::GitCommit {
                short_commit,
                subject,
                author,
                date,
                refs,
                ..
            } => {
                let refs = refs.trim();
                if refs.is_empty() {
                    Cow::Owned(format!("[{short_commit}] - {subject} ({date}) <{author}>"))
                } else {
                    Cow::Owned(format!(
                        "[{short_commit}] - ({refs}) {subject} ({date}) <{author}>"
                    ))
                }
            }
            Self::Stdin(text) => Cow::Borrowed(text.as_str()),
            Self::Message(text) => Cow::Borrowed(text.as_str()),
        }
    }

    pub fn preview_path(&self) -> Option<&str> {
        match self {
            Self::Path(path) | Self::Grep { path, .. } | Self::GitHistory { path, .. } => {
                Some(path.as_str())
            }
            Self::GitBranch { .. } | Self::GitCommit { .. } | Self::Stdin(_) | Self::Message(_) => {
                None
            }
        }
    }

    pub fn grep_line(&self) -> Option<usize> {
        match self {
            Self::Grep { line, .. } | Self::GitHistory { line, .. } => Some(*line),
            _ => None,
        }
    }

    pub fn git_history_commit(&self) -> Option<&str> {
        match self {
            Self::GitHistory { commit, .. } => Some(commit.as_str()),
            Self::GitCommit { commit, .. } => Some(commit.as_str()),
            _ => None,
        }
    }

    pub fn git_branch_name(&self) -> Option<&str> {
        match self {
            Self::GitBranch { branch, .. } => Some(branch.as_str()),
            _ => None,
        }
    }

    pub fn git_branch_is_head(&self) -> bool {
        matches!(self, Self::GitBranch { is_head: true, .. })
    }

    pub fn is_content_search_item(&self) -> bool {
        matches!(self, Self::Grep { .. } | Self::GitHistory { .. })
    }

    pub fn is_stdin(&self) -> bool {
        matches!(self, Self::Stdin(_))
    }

    pub fn content_match_column(&self, match_indices: &[u32]) -> Option<usize> {
        match self {
            Self::Grep { .. } => crate::text::find_first_match_column_in_grep_result(
                self.display_text().as_ref(),
                match_indices,
            ),
            Self::GitHistory {
                commit, path, line, ..
            } => {
                let prefix = format!(
                    "{}: {}:{}:",
                    commit,
                    path.replace(HISTORY_PATH_SEPARATOR, "/"),
                    line
                );
                match_indices
                    .iter()
                    .find(|&&idx| idx as usize >= prefix.chars().count())
                    .map(|idx| (*idx as usize) - prefix.chars().count() + 1)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub item: SearchItem,
    pub indices: Vec<u32>,
    pub column: Option<usize>,
}
