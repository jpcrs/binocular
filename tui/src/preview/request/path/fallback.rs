use crate::preview::{binary, encoding, rich_text::create_rich_text_document, PreviewContent};
use ratatui::text::Text;
use std::fs;
use std::path::Path;

pub(crate) enum FileContent {
    Text(String),
    Binary,
    ReadError,
}

pub(crate) fn build_text_or_binary_preview(path: &Path) -> PreviewContent {
    match read_file_content(path) {
        FileContent::Text(text) => PreviewContent::RichText(create_rich_text_document(text, path)),
        FileContent::Binary => PreviewContent::PlainText(binary::generate_preview(path)),
        FileContent::ReadError => PreviewContent::PlainText(Text::from("Error opening file")),
    }
}

fn read_file_content(path: &Path) -> FileContent {
    match fs::read_to_string(path) {
        Ok(content) => {
            if content.as_bytes().contains(&0) {
                FileContent::Binary
            } else {
                FileContent::Text(content)
            }
        }
        Err(_) => read_non_utf8_file(path),
    }
}

fn read_non_utf8_file(path: &Path) -> FileContent {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return FileContent::ReadError,
    };

    if let Some(utf8_content) = encoding::try_decode_utf16(&bytes) {
        return FileContent::Text(utf8_content);
    }

    let likely_binary = bytes.contains(&0)
        || crate::text::proportion_of_printable_ascii_characters(&bytes)
            < crate::text::PRINTABLE_ASCII_THRESHOLD;
    if likely_binary {
        return FileContent::Binary;
    }

    FileContent::Text(String::from_utf8_lossy(&bytes).into_owned())
}
