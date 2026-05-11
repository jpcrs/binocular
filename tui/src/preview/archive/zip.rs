//! ZIP-family archive preview support.

use crate::preview::archive::listing::{render_entries, ArchiveEntry, MAX_ENTRIES_SHOWN};
use crate::preview::doc::PreviewDoc;
use ratatui::style::Color;
use ratatui::text::Text;
use std::path::Path;

pub fn preview_zip(path: &Path) -> Text<'static> {
    let mut doc = PreviewDoc::new();

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            doc.push_section("Error");
            doc.push_field("Message", e.to_string(), Color::Red);
            return doc.into_text();
        }
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            doc.push_section("Error");
            doc.push_field("Message", e.to_string(), Color::Red);
            return doc.into_text();
        }
    };

    let total = archive.len();
    let mut entries: Vec<ArchiveEntry> = Vec::with_capacity(total.min(MAX_ENTRIES_SHOWN + 1));

    // SECURITY: cap iteration to avoid CPU DoS on ZIPs with millions of entries.
    let iter_limit = total.min(MAX_ENTRIES_SHOWN + 1);
    for i in 0..iter_limit {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        entries.push(ArchiveEntry {
            name: entry.name().to_string(),
            is_dir: entry.is_dir(),
            uncompressed: entry.size(),
        });
    }

    render_entries(
        &mut doc,
        &entries,
        total,
        "Archive Info",
        "ZIP / JAR / Office",
    );
    doc.into_text()
}
