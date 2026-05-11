//! PDF preview: extracts and displays text content from PDF files.

use crate::preview::doc::PreviewDoc;
use ratatui::style::Color;
use ratatui::text::Text;
use std::path::Path;

mod extract;

const MAX_PREVIEW_PAGES: u32 = 10;

const MAX_LINE_DISPLAY_CHARS: usize = 500;

pub fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

pub fn generate_preview(path: &Path) -> Text<'static> {
    let mut doc = PreviewDoc::new();

    if let Ok(meta) = std::fs::metadata(path) {
        use crate::preview::doc::format_file_size;
        doc.push_section("File Info");
        doc.push_field("Size", format_file_size(meta.len()), Color::White);
        doc.push_blank_line();
    }

    let doc_result = lopdf::Document::load(path);
    let pdf = match doc_result {
        Ok(d) => d,
        Err(e) => {
            doc.push_section("Error");
            doc.push_field("Message", e.to_string(), Color::Red);
            return doc.into_text();
        }
    };

    let page_count = pdf.get_pages().len() as u32;
    doc.push_section("Document Info");
    doc.push_field("Pages", page_count.to_string(), Color::White);

    if let Ok(info) = pdf
        .trailer
        .get(b"Info")
        .and_then(|o| pdf.get_object(o.as_reference()?))
    {
        if let Ok(dict) = info.as_dict() {
            for (key, label) in &[
                (b"Title" as &[u8], "Title"),
                (b"Author", "Author"),
                (b"Subject", "Subject"),
                (b"Creator", "Creator"),
                (b"Producer", "Producer"),
                (b"CreationDate", "Created"),
            ] {
                if let Ok(val) = dict.get(key) {
                    if let Ok(s) = val.as_str() {
                        let decoded = extract::decode_pdf_string(s);
                        if !decoded.is_empty() {
                            doc.push_field(label, decoded, Color::White);
                        }
                    }
                }
            }
        }
    }

    doc.push_blank_line();

    let pages_to_show = page_count.min(MAX_PREVIEW_PAGES);
    doc.push_section(Box::leak(
        format!("Content (first {} of {} pages)", pages_to_show, page_count).into_boxed_str(),
    ));

    let page_ids: Vec<(u32, lopdf::ObjectId)> = pdf.get_pages().into_iter().collect();
    let mut page_ids_sorted: Vec<(u32, lopdf::ObjectId)> = page_ids;
    page_ids_sorted.sort_by_key(|(n, _)| *n);

    for (page_num, page_id) in page_ids_sorted.iter().take(pages_to_show as usize) {
        let page_label = Box::leak(format!("Page {}", page_num).into_boxed_str()) as &'static str;
        doc.push_section(page_label);

        match extract::extract_page_text(&pdf, *page_id) {
            Ok(text) if text.trim().is_empty() => {
                doc.push_muted_italic("   (no extractable text — possibly image-based)");
            }
            Ok(text) => {
                for line in text.lines().filter(|l| !l.trim().is_empty()) {
                    let (display, _) =
                        crate::text::truncate_str_chars(line.trim(), MAX_LINE_DISPLAY_CHARS);
                    doc.push_field("  ", display.to_string(), Color::White);
                }
            }
            Err(_) => {
                doc.push_muted_italic("   (could not extract text from this page)");
            }
        }
        doc.push_blank_line();
    }

    if page_count > MAX_PREVIEW_PAGES {
        doc.push_muted_italic(format!(
            "   … {} more pages not shown",
            page_count - MAX_PREVIEW_PAGES
        ));
    }

    doc.into_text()
}

pub fn extract_all_text(path: &Path) -> lopdf::Result<Vec<String>> {
    extract::extract_all_text(path)
}
