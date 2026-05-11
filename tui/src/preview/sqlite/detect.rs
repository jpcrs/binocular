//! SQLite file detection.

use std::io::Read;
use std::path::Path;

const SQLITE_MAGIC: &[u8] = b"SQLite format 3\x00";

pub fn is_sqlite(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !matches!(
        ext.as_str(),
        "db" | "sqlite" | "sqlite3" | "db3" | "s3db" | "sl3"
    ) {
        return false;
    }

    check_magic(path)
}

fn check_magic(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 16];
    file.read_exact(&mut buf).is_ok() && buf == SQLITE_MAGIC[..16]
}
