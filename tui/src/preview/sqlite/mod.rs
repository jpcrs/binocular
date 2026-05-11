use crate::preview::doc::{format_file_size, PreviewDoc};
use crate::preview::sqlite::document::render_table_document;
use crate::preview::sqlite::schema::{list_objects, DbObject};
use ratatui::style::Color;
use ratatui::text::Text;
use std::path::Path;

mod detect;
mod document;
mod schema;

const MAX_TABLES_DETAIL: usize = 20;

const SAMPLE_ROWS: usize = 5;

pub fn is_sqlite(path: &Path) -> bool {
    detect::is_sqlite(path)
}

pub fn generate_preview(path: &Path) -> Text<'static> {
    let mut doc = PreviewDoc::new();

    if let Ok(meta) = std::fs::metadata(path) {
        doc.push_section("File Info");
        doc.push_field("Size", format_file_size(meta.len()), Color::White);
        doc.push_blank_line();
    }

    let conn = match rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(e) => {
            doc.push_section("Error");
            doc.push_field("Message", e.to_string(), Color::Red);
            return doc.into_text();
        }
    };

    let page_size: i64 = conn
        .pragma_query_value(None, "page_size", |r| r.get(0))
        .unwrap_or(0);
    let page_count: i64 = conn
        .pragma_query_value(None, "page_count", |r| r.get(0))
        .unwrap_or(0);
    let db_size = page_size * page_count;

    doc.push_section("Database Info");
    doc.push_field("Page size", format!("{} bytes", page_size), Color::White);
    if db_size > 0 {
        doc.push_field("DB size", format_file_size(db_size as u64), Color::White);
    }
    doc.push_blank_line();

    let objects = match list_objects(&conn) {
        Ok(o) => o,
        Err(e) => {
            doc.push_section("Error");
            doc.push_field("Message", e.to_string(), Color::Red);
            return doc.into_text();
        }
    };

    let tables: Vec<&DbObject> = objects.iter().filter(|o| o.kind == "table").collect();
    let views: Vec<&DbObject> = objects.iter().filter(|o| o.kind == "view").collect();

    doc.push_section("Schema");
    doc.push_field("Tables", tables.len().to_string(), Color::White);
    doc.push_field("Views", views.len().to_string(), Color::White);
    doc.push_blank_line();

    for (i, obj) in tables.iter().take(MAX_TABLES_DETAIL).enumerate() {
        if i > 0 {
            doc.push_blank_line();
        }
        render_table_document(&conn, &mut doc, obj, SAMPLE_ROWS);
    }

    if tables.len() > MAX_TABLES_DETAIL {
        doc.push_blank_line();
        doc.push_muted_italic(format!(
            "   … {} more tables",
            tables.len() - MAX_TABLES_DETAIL
        ));
    }

    if !views.is_empty() {
        doc.push_blank_line();
        doc.push_section("Views");
        for view in &views {
            doc.push_field("  view", view.name.clone(), Color::Cyan);
        }
    }

    doc.into_text()
}
