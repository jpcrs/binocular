use super::ansi::parse_ansi_text;
use crate::preview::PreviewContent;
use crate::search::sources::git::read_branch_preview;
use ratatui::text::Text;

pub(crate) fn build_git_branch_preview(repo_root: &str, branch: &str) -> PreviewContent {
    match read_branch_preview(std::path::Path::new(repo_root), branch) {
        Ok(text) => PreviewContent::PlainText(parse_ansi_text(text)),
        Err(err) => {
            PreviewContent::PlainText(Text::from(format!("Failed to load branch preview: {err}")))
        }
    }
}
