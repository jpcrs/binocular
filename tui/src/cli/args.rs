use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::search::sources::git::GitSearchScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Plain,
    Jsonl,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct GlobalArgs {
    /// Headless mode: print results to stdout without opening the TUI
    #[arg(short = 'H', long)]
    pub headless: bool,

    /// Format used when printing interactive selections to stdout
    #[arg(long, value_enum, default_value_t = OutputFormat::Plain)]
    pub output_format: OutputFormat,

    /// Preview command (supports '{}' for full item, '{0}', '{1}', ... for delimiter-split parts)
    #[arg(long)]
    pub preview: Option<String>,

    /// Delimiter used to split result items into numbered parameters for --preview (default: ":")
    #[arg(long, default_value = ":")]
    pub delimiter: String,

    /// Split each stdin line into multiple items using this delimiter.
    /// For example, --split "," turns "a,b,c" into three separate items.
    #[arg(short = 's', long)]
    pub split: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Search full paths (default command)
    Path(SearchCommandArgs),
    /// Search file names only
    Files(SearchCommandArgs),
    /// Search file contents
    #[command(alias = "grep")]
    Content(ContentCommandArgs),
    /// Search directories only
    Dirs(SearchCommandArgs),
    /// Open the structured log viewer from stdin
    Log(LogCommandArgs),
    /// Open a direct diff preview for two files
    Diff(DiffCommandArgs),
    /// Git-backed search commands
    Git(GitCommandArgs),
}

#[derive(ClapArgs, Debug, Clone, Default)]
pub struct SearchCommandArgs {
    /// Initial search query (pre-populates the search bar)
    #[arg(index = 1)]
    pub query: Option<String>,

    /// Directory to search in (can be specified multiple times for multiple roots)
    #[arg(short = 'l', long, value_name = "DIR", action = ArgAction::Append)]
    pub location: Vec<PathBuf>,

    /// Exact match: every search token must appear as a contiguous substring
    #[arg(short, long)]
    pub exact: bool,

    /// Skip hidden files and directories (default: hidden files are included)
    #[arg(long = "no-hidden")]
    pub no_hidden: bool,

    /// Do not respect .gitignore files (default: .gitignore is respected)
    #[arg(long = "no-git-ignore")]
    pub no_git_ignore: bool,

    /// Do not respect .ignore files (default: .ignore is respected)
    #[arg(long = "no-ignore")]
    pub no_ignore: bool,

    /// Do not apply the built-in ignore list (node_modules, target, .git, etc.)
    #[arg(long = "no-default-ignore-dirs")]
    pub no_default_ignore_dirs: bool,
}

#[derive(ClapArgs, Debug, Clone, Default)]
pub struct ContentCommandArgs {
    #[command(flatten)]
    pub search: SearchCommandArgs,

    /// In content mode, also extract and search text inside PDF files
    #[arg(long = "search-pdf")]
    pub search_pdf: bool,
}

#[derive(ClapArgs, Debug, Clone, Default)]
pub struct LogCommandArgs {
    /// Log file(s) to tail. If omitted, reads from stdin.
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct DiffCommandArgs {
    pub left: PathBuf,
    pub right: PathBuf,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct GitCommandArgs {
    #[command(subcommand)]
    pub command: GitSubcommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum GitSubcommand {
    /// Search the committed history of one tracked file
    History(GitHistoryCommandArgs),
    /// Search local branches
    Branches(GitListCommandArgs),
    /// Search commits on the current branch
    #[command(alias = "logs")]
    Commits(GitListCommandArgs),
}

#[derive(ClapArgs, Debug, Clone)]
pub struct GitHistoryCommandArgs {
    pub file: PathBuf,

    /// Initial search query (pre-populates the search bar)
    #[arg(index = 2)]
    pub query: Option<String>,

    /// Exact match: every search token must appear as a contiguous substring
    #[arg(short, long)]
    pub exact: bool,
}

#[derive(ClapArgs, Debug, Clone, Default)]
pub struct GitListCommandArgs {
    /// Initial search query (pre-populates the search bar)
    #[arg(index = 1)]
    pub query: Option<String>,

    /// Exact match: every search token must appear as a contiguous substring
    #[arg(short, long)]
    pub exact: bool,
}

#[derive(Debug, Clone)]
pub struct Args {
    pub query: Option<String>,
    pub location: Vec<PathBuf>,
    pub dir_only: bool,
    pub file_name: bool,
    pub full_path: bool,
    pub content: bool,
    pub exact: bool,
    pub no_hidden: bool,
    pub no_git_ignore: bool,
    pub no_ignore: bool,
    pub no_default_ignore_dirs: bool,
    pub search_pdf: bool,
    pub git_history: Option<PathBuf>,
    pub git_branches: bool,
    pub git_commits: bool,
    pub headless: bool,
    pub diff: Option<Vec<PathBuf>>,
    pub output_format: OutputFormat,
    pub stdin: bool,
    pub git_search_scope: Option<GitSearchScope>,
    pub preview: Option<String>,
    pub delimiter: String,
    pub split: Option<String>,
    pub log: bool,
    pub log_files: Vec<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            query: None,
            location: vec![],
            dir_only: false,
            file_name: false,
            full_path: false,
            content: false,
            exact: false,
            no_hidden: false,
            no_git_ignore: false,
            no_ignore: false,
            no_default_ignore_dirs: false,
            search_pdf: false,
            git_history: None,
            git_branches: false,
            git_commits: false,
            headless: false,
            diff: None,
            output_format: OutputFormat::Plain,
            stdin: false,
            git_search_scope: None,
            preview: None,
            delimiter: ":".to_string(),
            split: None,
            log: false,
            log_files: vec![],
        }
    }
}

impl Cli {
    pub fn into_args(self) -> Args {
        let mut args = Args {
            headless: self.global.headless,
            output_format: self.global.output_format,
            preview: self.global.preview,
            delimiter: self.global.delimiter,
            split: self.global.split,
            ..Args::default()
        };

        match self.command {
            None => {
                args.full_path = true;
            }
            Some(Command::Path(cmd)) => {
                apply_search_command(&mut args, cmd);
                args.full_path = true;
            }
            Some(Command::Files(cmd)) => {
                apply_search_command(&mut args, cmd);
                args.file_name = true;
            }
            Some(Command::Content(cmd)) => {
                apply_search_command(&mut args, cmd.search);
                args.content = true;
                args.search_pdf = cmd.search_pdf;
            }
            Some(Command::Dirs(cmd)) => {
                apply_search_command(&mut args, cmd);
                args.dir_only = true;
            }
            Some(Command::Log(cmd)) => {
                args.log = true;
                args.log_files = cmd.files;
            }
            Some(Command::Diff(cmd)) => {
                args.diff = Some(vec![cmd.left, cmd.right]);
            }
            Some(Command::Git(cmd)) => match cmd.command {
                GitSubcommand::History(cmd) => {
                    args.git_history = Some(cmd.file);
                    args.query = cmd.query;
                    args.exact = cmd.exact;
                }
                GitSubcommand::Branches(cmd) => {
                    args.git_branches = true;
                    args.query = cmd.query;
                    args.exact = cmd.exact;
                }
                GitSubcommand::Commits(cmd) => {
                    args.git_commits = true;
                    args.query = cmd.query;
                    args.exact = cmd.exact;
                }
            },
        }

        args
    }
}

fn apply_search_command(args: &mut Args, cmd: SearchCommandArgs) {
    args.query = cmd.query;
    args.location = cmd.location;
    args.exact = cmd.exact;
    args.no_hidden = cmd.no_hidden;
    args.no_git_ignore = cmd.no_git_ignore;
    args.no_ignore = cmd.no_ignore;
    args.no_default_ignore_dirs = cmd.no_default_ignore_dirs;
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn grep_alias_maps_to_content_command() {
        let cli = Cli::parse_from(["binocular", "grep", "needle"]);
        let args = cli.into_args();
        assert!(args.content);
        assert_eq!(args.query.as_deref(), Some("needle"));
    }

    #[test]
    fn git_logs_alias_maps_to_commits_command() {
        let cli = Cli::parse_from(["binocular", "git", "logs", "needle"]);
        let args = cli.into_args();
        assert!(args.git_commits);
        assert_eq!(args.query.as_deref(), Some("needle"));
    }
}
