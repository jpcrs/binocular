use super::preview::read_history_blob;
use super::scope::{GitSearchMode, GitSearchScope};
use crate::infra::channel::{BatchSender, Sender};
use crate::search::types::SearchItem;
use anyhow::Context;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const GIT_BATCH_SIZE: usize = 128;
pub const HISTORY_PATH_SEPARATOR: char = '\u{1f}';
const HISTORY_PATH_REPLACEMENT: char = '\u{fffd}';
pub const CURRENT_COMMIT_REF: &str = "HEAD";
const FIELD_SEPARATOR: char = '\u{1e}';

pub fn spawn_git_searcher(
    scope: GitSearchScope,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        if let Err(err) = stream_git_items(scope, stop, tx.clone()) {
            let _ = tx.send(vec![SearchItem::message(err.to_string())]);
        }
    })
}

pub fn sanitize_path_field(path: &str) -> String {
    path.replace(
        HISTORY_PATH_SEPARATOR,
        &HISTORY_PATH_REPLACEMENT.to_string(),
    )
}

pub fn is_current_commit(commit: &str) -> bool {
    commit == CURRENT_COMMIT_REF
}

fn stream_git_items(
    scope: GitSearchScope,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> anyhow::Result<()> {
    match scope.mode {
        GitSearchMode::History { file } => {
            let display_path = scope
                .display_path
                .unwrap_or_else(|| file.to_string_lossy().into_owned());
            stream_history(scope.repo_root, file, display_path, stop, tx)
        }
        GitSearchMode::Branches => stream_branches(scope.repo_root, stop, tx),
        GitSearchMode::Commits => stream_commits(scope.repo_root, stop, tx),
    }
}

fn stream_history(
    repo_root: PathBuf,
    file: PathBuf,
    display_path: String,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> anyhow::Result<()> {
    let mut child = Command::new("git")
        .arg("log")
        .arg("--format=%H")
        .arg("--follow")
        .arg("--")
        .arg(&file)
        .current_dir(&repo_root)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run git log for history search")?;

    let stdout = child.stdout.take().context("git log stdout unavailable")?;
    let mut batcher = BatchSender::new(tx, GIT_BATCH_SIZE);

    for line in BufReader::new(stdout).lines() {
        if stop.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(());
        }

        let hash = line.context("failed to read git log output")?;
        let hash = hash.trim();
        if hash.is_empty() {
            continue;
        }

        let blob = match read_history_blob(&repo_root, hash, &file) {
            Ok(blob) => blob,
            Err(_) => continue,
        };
        if blob.contains(&0) {
            continue;
        }

        push_history_lines(&mut batcher, hash, &display_path, &blob, &stop);
    }

    wait_for_success(child, stop, "git log")
}

fn stream_branches(
    repo_root: PathBuf,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> anyhow::Result<()> {
    let format = format!(
        "%(HEAD){sep}%(refname:short){sep}%(objectname:short){sep}%(contents:subject){sep}%(committerdate:relative)",
        sep = FIELD_SEPARATOR
    );
    let mut child = Command::new("git")
        .arg("for-each-ref")
        .arg("--sort=-committerdate")
        .arg(format!("--format={format}"))
        .arg("refs/heads")
        .current_dir(&repo_root)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run git for-each-ref for branches")?;

    let stdout = child
        .stdout
        .take()
        .context("git branch stdout unavailable")?;
    let mut batcher = BatchSender::new(tx, GIT_BATCH_SIZE);
    for line in BufReader::new(stdout).lines() {
        if stop.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(());
        }

        let line = line.context("failed to read git branch output")?;
        let mut parts = line.splitn(5, FIELD_SEPARATOR);
        let head_marker = parts.next().unwrap_or_default().trim();
        let branch = parts.next().unwrap_or_default().trim();
        let commit = parts.next().unwrap_or_default().trim();
        let subject = parts.next().unwrap_or_default().trim();
        let relative_time = parts.next().unwrap_or_default().trim();
        if branch.is_empty() {
            continue;
        }
        batcher.push(SearchItem::git_branch(
            branch,
            commit,
            subject,
            head_marker == "*",
            relative_time,
        ));
    }

    wait_for_success(child, stop, "git for-each-ref")
}

fn stream_commits(
    repo_root: PathBuf,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> anyhow::Result<()> {
    let format = format!(
        "%H{sep}%h{sep}%s{sep}%an{sep}%ar{sep}%D",
        sep = FIELD_SEPARATOR
    );
    let mut child = Command::new("git")
        .arg("log")
        .arg("--date=short")
        .arg(format!("--format={format}"))
        .current_dir(&repo_root)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run git log for commits")?;

    let stdout = child
        .stdout
        .take()
        .context("git commit log stdout unavailable")?;
    let mut batcher = BatchSender::new(tx, GIT_BATCH_SIZE);
    for line in BufReader::new(stdout).lines() {
        if stop.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(());
        }

        let line = line.context("failed to read git commit output")?;
        let mut parts = line.splitn(6, FIELD_SEPARATOR);
        let commit = parts.next().unwrap_or_default().trim();
        let short_commit = parts.next().unwrap_or_default().trim();
        let subject = parts.next().unwrap_or_default().trim();
        let author = parts.next().unwrap_or_default().trim();
        let date = parts.next().unwrap_or_default().trim();
        let refs = parts.next().unwrap_or_default().trim();
        if commit.is_empty() {
            continue;
        }
        batcher.push(SearchItem::git_commit(
            commit,
            short_commit,
            subject,
            author,
            date,
            refs,
        ));
    }

    wait_for_success(child, stop, "git log")
}

fn push_history_lines<S: Sender<Vec<SearchItem>>>(
    batcher: &mut BatchSender<SearchItem, S>,
    commit: &str,
    display_path: &str,
    blob: &[u8],
    stop: &AtomicBool,
) {
    let sanitized_path = sanitize_path_field(display_path);
    let mut line_number = 1usize;
    for line in BufReader::new(blob).split(b'\n') {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let Ok(mut line) = line else { break };
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        let text = String::from_utf8_lossy(&line).into_owned();
        batcher.push(SearchItem::history_line(
            commit,
            &sanitized_path,
            line_number,
            text,
        ));
        line_number += 1;
    }
}

fn wait_for_success(
    mut child: std::process::Child,
    stop: Arc<AtomicBool>,
    command_name: &str,
) -> anyhow::Result<()> {
    let status = child
        .wait()
        .with_context(|| format!("failed to wait for {command_name}"))?;
    if !status.success() && !stop.load(Ordering::Relaxed) {
        anyhow::bail!("{command_name} exited with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_path_field_replaces_internal_separator() {
        let sanitized = sanitize_path_field("dir\u{1f}file.txt");
        assert!(!sanitized.contains(HISTORY_PATH_SEPARATOR));
    }
}
