use super::ansi::parse_ansi_text;
use crate::preview::PreviewContent;
use crate::search::sources::git::read_commit_preview;
use ratatui::text::Text;

pub(crate) fn build_git_commit_preview(repo_root: &str, commit: &str) -> PreviewContent {
    match read_commit_preview(std::path::Path::new(repo_root), commit) {
        Ok(text) => PreviewContent::PlainText(parse_ansi_text(text)),
        Err(err) => {
            PreviewContent::PlainText(Text::from(format!("Failed to load commit preview: {err}")))
        }
    }
}
