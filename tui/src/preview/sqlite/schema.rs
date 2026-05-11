//! SQLite schema and sample-row queries.

pub const MAX_COL_WIDTH: usize = 30;

#[derive(Clone, Debug)]
pub struct DbObject {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub typ: String,
    pub notnull: bool,
    pub pk: bool,
}

pub fn list_objects(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<DbObject>> {
    let mut stmt = conn.prepare(
        "SELECT name, type FROM sqlite_master WHERE type IN ('table','view') ORDER BY type, name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DbObject {
            name: row.get(0)?,
            kind: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn get_columns(conn: &rusqlite::Connection, table: &str) -> rusqlite::Result<Vec<Column>> {
    let safe_name = table.replace('"', "\"\"");
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", safe_name))?;
    let cols = stmt.query_map([], |row| {
        Ok(Column {
            name: row.get::<_, String>(1)?,
            typ: row.get::<_, String>(2).unwrap_or_default(),
            notnull: row.get::<_, i32>(3).unwrap_or(0) != 0,
            pk: row.get::<_, i32>(5).unwrap_or(0) != 0,
        })
    })?;
    cols.collect()
}

pub fn get_sample_rows(
    conn: &rusqlite::Connection,
    table: &str,
    columns: &[Column],
    limit: usize,
) -> rusqlite::Result<Vec<Vec<String>>> {
    let safe = table.replace('"', "\"\"");
    let query = format!("SELECT * FROM \"{}\" LIMIT {}", safe, limit);
    let mut stmt = conn.prepare(&query)?;
    let col_count = columns.len();

    let rows = stmt.query_map([], |row| {
        let mut cells = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val: String = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => "NULL".to_string(),
                rusqlite::types::ValueRef::Integer(n) => n.to_string(),
                rusqlite::types::ValueRef::Real(f) => format!("{:.4}", f),
                rusqlite::types::ValueRef::Text(s) => String::from_utf8_lossy(s)
                    .chars()
                    .take(MAX_COL_WIDTH)
                    .collect(),
                rusqlite::types::ValueRef::Blob(b) => format!("<blob {} bytes>", b.len()),
            };
            cells.push(val);
        }
        Ok(cells)
    })?;
    rows.collect()
}
