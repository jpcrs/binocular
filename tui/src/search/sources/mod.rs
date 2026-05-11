pub mod filesystem;
pub mod git;
pub mod stdin;

pub use filesystem::spawn_searcher_with_config;
pub use git::spawn_git_searcher;
pub use stdin::spawn_stdin_searcher;
