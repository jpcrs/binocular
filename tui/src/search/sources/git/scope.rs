use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitSearchMode {
    History { file: PathBuf },
    Branches,
    Commits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSearchScope {
    pub repo_root: PathBuf,
    pub mode: GitSearchMode,
    pub display_path: Option<String>,
}

pub fn resolve_history_scope(path: &Path) -> anyhow::Result<GitSearchScope> {
    let path = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    };

    let repo_root = git_repo_root(&path)?;
    let file_path = if path.is_absolute() {
        path.clone()
    } else {
        repo_root.join(&path)
    };
    let relative = file_path
        .strip_prefix(&repo_root)
        .unwrap_or(&file_path)
        .to_path_buf();
    let display_path = relative.to_string_lossy().replace('\\', "/");

    Ok(GitSearchScope {
        repo_root,
        mode: GitSearchMode::History { file: relative },
        display_path: Some(display_path),
    })
}

pub fn resolve_repo_scope(
    start: Option<&Path>,
    mode: GitSearchMode,
) -> anyhow::Result<GitSearchScope> {
    let start = start.unwrap_or_else(|| Path::new("."));
    let repo_root = git_repo_root(start)?;
    Ok(GitSearchScope {
        repo_root,
        mode,
        display_path: None,
    })
}

pub(crate) fn git_repo_root(path: &Path) -> anyhow::Result<PathBuf> {
    let cwd = repo_lookup_dir(path);
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(cwd)
        .output()
        .context("failed to determine git repository root")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git rev-parse --show-toplevel failed: {}",
            if_empty_then(stderr.trim(), "not a git repository")
        );
    }

    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

fn repo_lookup_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.to_path_buf();
    }

    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

pub(crate) fn if_empty_then<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_filename_repo_lookup_uses_current_directory() {
        assert_eq!(
            repo_lookup_dir(Path::new("Architecture.md")),
            PathBuf::from(".")
        );
        assert_eq!(
            repo_lookup_dir(Path::new("./Architecture.md")),
            PathBuf::from(".")
        );
    }
}
