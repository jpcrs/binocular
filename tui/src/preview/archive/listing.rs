//! Shared archive preview document helpers.

use crate::preview::doc::{format_file_size, PreviewDoc};
use ratatui::style::Color;
use ratatui::text::Text;

pub const MAX_ENTRIES_SHOWN: usize = 200;

#[derive(Clone, Debug)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub uncompressed: u64,
}

pub fn render_entries(
    doc: &mut PreviewDoc,
    entries: &[ArchiveEntry],
    total: usize,
    section: &'static str,
    kind_label: &'static str,
) {
    let dirs = entries.iter().filter(|e| e.is_dir).count();
    let files = entries.iter().filter(|e| !e.is_dir).count();
    let total_uncompressed: u64 = entries.iter().map(|e| e.uncompressed).sum();

    doc.push_section(section);
    doc.push_field("Format", kind_label, Color::Green);
    doc.push_field("Files", files.to_string(), Color::White);
    doc.push_field("Directories", dirs.to_string(), Color::White);
    if total_uncompressed > 0 {
        doc.push_field(
            "Uncompressed size",
            format_file_size(total_uncompressed),
            Color::White,
        );
    }
    doc.push_blank_line();

    doc.push_section("Contents");
    let shown = entries.len().min(MAX_ENTRIES_SHOWN);
    for entry in entries.iter().take(shown) {
        render_entry(doc, &entry.name, entry.is_dir, entry.uncompressed);
    }
    if total > shown {
        doc.push_muted_italic(format!("   … {} more entries", total - shown));
    }
}

pub fn error_text(msg: String) -> Text<'static> {
    let mut doc = PreviewDoc::new();
    doc.push_section("Error");
    doc.push_field("Message", msg, Color::Red);
    doc.into_text()
}

fn render_entry(doc: &mut PreviewDoc, name: &str, is_dir: bool, size: u64) {
    let depth = name.trim_end_matches('/').matches('/').count();
    let indent = "  ".repeat(depth);
    let basename = name
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(name);

    if is_dir {
        doc.push_field(&format!("{}📁 {}/", indent, basename), "", Color::Blue);
    } else if size > 0 {
        doc.push_field(
            &format!("{}  {}", indent, basename),
            format_file_size(size),
            Color::DarkGray,
        );
    } else {
        doc.push_field(&format!("{}  {}", indent, basename), "", Color::White);
    }
}
