use super::scope::if_empty_then;
use anyhow::{bail, Context};
use std::path::Path;
use std::process::Command;

pub fn read_history_blob(
    repo_root: &Path,
    commit: &str,
    file_path: &Path,
) -> anyhow::Result<Vec<u8>> {
    let object = format!("{}:{}", commit, file_path.to_string_lossy());
    let output = Command::new("git")
        .arg("show")
        .arg(&object)
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("failed to read historical blob for {object}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git show failed for {object}: {}",
            if_empty_then(stderr.trim(), "unknown git error")
        );
    }

    Ok(output.stdout)
}

pub fn read_branch_preview(repo_root: &Path, branch: &str) -> anyhow::Result<String> {
    run_git_text(
        repo_root,
        &[
            "log",
            "--color=always",
            "--decorate",
            "--stat",
            "--max-count=25",
            branch,
        ],
        &format!("failed to read branch preview for {branch}"),
    )
}

pub fn read_commit_preview(repo_root: &Path, commit: &str) -> anyhow::Result<String> {
    run_git_text(
        repo_root,
        &[
            "show",
            "--color=always",
            "--stat",
            "--patch",
            "--decorate",
            commit,
        ],
        &format!("failed to read commit preview for {commit}"),
    )
}

fn run_git_text(repo_root: &Path, args: &[&str], context: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .context(context.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{}: {}",
            context,
            if_empty_then(stderr.trim(), "unknown git error")
        );
    }

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = "No output".to_string();
    }
    Ok(text)
}
