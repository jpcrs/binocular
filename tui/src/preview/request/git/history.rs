use crate::preview::encoding;
use crate::preview::{create_rich_text_document, PreviewContent};
use crate::search::sources::git::read_history_blob;
use ratatui::text::Text;

pub(crate) fn build_history_preview(repo_root: &str, commit: &str, path: &str) -> PreviewContent {
    match read_history_blob(
        std::path::Path::new(repo_root),
        commit,
        std::path::Path::new(path),
    ) {
        Ok(bytes) => PreviewContent::RichText(create_rich_text_document(
            decode_history_blob(bytes),
            std::path::Path::new(path),
        )),
        Err(err) => PreviewContent::PlainText(Text::from(format!(
            "Failed to load git history preview: {err}"
        ))),
    }
}

fn decode_history_blob(bytes: Vec<u8>) -> String {
    if let Some(decoded) = encoding::try_decode_utf16(&bytes) {
        return decoded;
    }

    if bytes.contains(&0) {
        return "Binary historical blobs are not previewable".to_string();
    }

    String::from_utf8_lossy(&bytes).into_owned()
}
