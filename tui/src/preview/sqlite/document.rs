//! SQLite preview document helpers.

use crate::preview::doc::PreviewDoc;
use crate::preview::sqlite::schema::{
    get_columns, get_sample_rows, Column, DbObject, MAX_COL_WIDTH,
};
use ratatui::style::Color;

pub fn render_table_document(
    conn: &rusqlite::Connection,
    doc: &mut PreviewDoc,
    obj: &DbObject,
    sample_rows: usize,
) {
    doc.push_section(Box::leak(format!("Table: {}", obj.name).into_boxed_str()));

    let count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM \"{}\"", obj.name.replace('"', "\"\"")),
            [],
            |r| r.get(0),
        )
        .unwrap_or(-1);
    if count >= 0 {
        doc.push_field("Rows", count.to_string(), Color::White);
    }

    let columns = match get_columns(conn, &obj.name) {
        Ok(c) => c,
        Err(_) => return,
    };

    if columns.is_empty() {
        return;
    }

    let col_summary: Vec<String> = columns
        .iter()
        .map(|c| {
            if c.pk {
                format!("{} {} 🔑", c.name, c.typ)
            } else if c.notnull {
                format!("{} {} !", c.name, c.typ)
            } else {
                format!("{} {}", c.name, c.typ)
            }
        })
        .collect();
    doc.push_field("Columns", col_summary.join("  |  "), Color::Cyan);

    if count == 0 {
        doc.push_muted_italic("   (empty table)");
        return;
    }

    let sample = match get_sample_rows(conn, &obj.name, &columns, sample_rows) {
        Ok(r) => r,
        Err(_) => return,
    };

    if sample.is_empty() {
        return;
    }

    doc.push_blank_line();
    render_row_table(doc, &columns, &sample);
}

fn render_row_table(doc: &mut PreviewDoc, columns: &[Column], rows: &[Vec<String>]) {
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let max_data = rows
                .iter()
                .map(|r| r.get(i).map(|s| s.len()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            col.name.len().max(max_data).min(MAX_COL_WIDTH)
        })
        .collect();

    let header: String = columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:<width$}", col.name, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    doc.push_field("   ", header, Color::Yellow);

    let sep: String = widths
        .iter()
        .map(|&w| "─".repeat(w))
        .collect::<Vec<_>>()
        .join("──");
    doc.push_field("   ", sep, Color::DarkGray);

    for row in rows {
        let formatted: String = row
            .iter()
            .enumerate()
            .map(|(i, val)| {
                let w = widths.get(i).copied().unwrap_or(10);
                format!("{:<width$}", val, width = w)
            })
            .collect::<Vec<_>>()
            .join("  ");
        doc.push_field("   ", formatted, Color::White);
    }
}
