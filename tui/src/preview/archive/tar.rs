//! TAR-family archive preview support.

use crate::preview::archive::listing::{
    error_text, render_entries, ArchiveEntry, MAX_ENTRIES_SHOWN,
};
use crate::preview::doc::PreviewDoc;
use ratatui::style::Color;
use ratatui::text::Text;
use std::io::Read;
use std::path::Path;

pub fn preview_tar_gz(path: &Path) -> Text<'static> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return error_text(e.to_string()),
    };
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    preview_tar_archive(&mut archive, ".tar.gz / .tgz")
}

pub fn preview_tar_bz2(path: &Path) -> Text<'static> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return error_text(e.to_string()),
    };
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    preview_tar_archive(&mut archive, ".tar.bz2")
}

pub fn preview_tar_xz(path: &Path) -> Text<'static> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return error_text(e.to_string()),
    };
    let decoder = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    preview_tar_archive(&mut archive, ".tar.xz")
}

pub fn preview_tar_raw(path: &Path) -> Text<'static> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return error_text(e.to_string()),
    };
    let mut archive = tar::Archive::new(file);
    preview_tar_archive(&mut archive, ".tar")
}

fn preview_tar_archive<R: Read>(
    archive: &mut tar::Archive<R>,
    kind_label: &'static str,
) -> Text<'static> {
    let mut doc = PreviewDoc::new();
    let mut entries: Vec<ArchiveEntry> = Vec::new();
    let mut total = 0usize;

    let Ok(iter) = archive.entries() else {
        doc.push_section("Error");
        doc.push_field("Message", "Could not read archive entries", Color::Red);
        return doc.into_text();
    };

    for entry_result in iter {
        total += 1;
        if entries.len() < MAX_ENTRIES_SHOWN + 1 {
            if let Ok(entry) = entry_result {
                let path_str = entry
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let is_dir = path_str.ends_with('/');
                let uncompressed = entry.header().size().unwrap_or(0);
                entries.push(ArchiveEntry {
                    name: path_str,
                    is_dir,
                    uncompressed,
                });
            }
        }
    }

    render_entries(&mut doc, &entries, total, "Archive Info", kind_label);
    doc.into_text()
}
