//! Binary preview file metadata rendering.

use crate::preview::doc::{format_file_size, format_unix_timestamp, PreviewDoc};
use ratatui::style::Color;
use std::fs;
use std::path::Path;

pub fn append_file_metadata(path: &Path, doc: &mut PreviewDoc) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    doc.push_section(super::SECTION_FILE_INFO);
    doc.push_field("Size", format_file_size(metadata.len()), Color::White);

    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            doc.push_field(
                "Modified",
                format_unix_timestamp(duration.as_secs()),
                Color::White,
            );
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        doc.push_field("Permissions", format!("{:o}", mode & 0o777), Color::White);
    }

    doc.push_blank_line();
}
