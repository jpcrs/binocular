use crate::cli::args::{Args, OutputFormat};
use crate::cli::Cli;
use crate::search::sources::git::{resolve_history_scope, resolve_repo_scope, GitSearchMode};
use crate::search::types::SearchConfig;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub headless: bool,
    pub output_format: OutputFormat,
    pub output_file: Option<PathBuf>,
    pub stdin: bool,
    pub log: bool,
    pub log_files: Vec<PathBuf>,
    pub diff: Option<[PathBuf; 2]>,
    pub preview_command: Option<String>,
    pub preview_delimiter: String,
    pub split: Option<String>,
}

impl RunConfig {
    pub fn from_args(args: &Args) -> Self {
        Self {
            headless: args.headless,
            output_format: args.output_format,
            output_file: args.output_file.clone(),
            stdin: args.stdin,
            log: args.log,
            log_files: args.log_files.clone(),
            diff: args.diff.as_ref().and_then(|paths| match paths.as_slice() {
                [left, right] => Some([left.clone(), right.clone()]),
                _ => None,
            }),
            preview_command: args.preview.clone(),
            preview_delimiter: args.delimiter.clone(),
            split: args.split.clone(),
        }
    }

    pub fn has_preview_command(&self) -> bool {
        self.preview_command.is_some()
    }

    pub fn with_stdin(&self, stdin: bool) -> Self {
        let mut config = self.clone();
        config.stdin = stdin;
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCli {
    pub run: RunConfig,
    pub search: SearchConfig,
}

impl ResolvedCli {
    pub fn from_cli(cli: Cli, stdin_is_piped: bool) -> anyhow::Result<Self> {
        let args = cli.into_args();
        let run = RunConfig::from_args(&args).with_stdin(stdin_is_piped);

        let git_search_scope = if let Some(path) = args.git_history.as_deref() {
            Some(resolve_history_scope(path)?)
        } else if args.git_branches {
            Some(resolve_repo_scope(
                args.location.first().map(|p| p.as_path()),
                GitSearchMode::Branches,
            )?)
        } else if args.git_commits {
            Some(resolve_repo_scope(
                args.location.first().map(|p| p.as_path()),
                GitSearchMode::Commits,
            )?)
        } else {
            None
        };

        let search = SearchConfig::from_args(&args).with_git_search_scope(git_search_scope);

        Ok(Self { run, search })
    }
}
