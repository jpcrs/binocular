mod analysis;
mod detect;
mod hex_dump;
mod metadata;
mod strings;

use crate::preview::doc::PreviewDoc;
use ratatui::text::Text;
use std::fs;
use std::io::Read;
use std::path::Path;

pub(super) const HEADER_READ_BYTES: usize = 512;
pub(super) const ENTROPY_SAMPLE_BYTES: usize = 1024 * 1024;
pub(super) const HEX_DUMP_BYTES: usize = 128;
pub(super) const STRINGS_SAMPLE_BYTES: usize = 8192;
pub(super) const MIN_PRINTABLE_STRING_LEN: usize = 6;
pub(super) const MAX_PRINTABLE_STRINGS_SHOWN: usize = 15;
pub(super) const MAX_STRING_PREVIEW_LEN: usize = 60;

pub(super) const SECTION_FILE_INFO: &str = "File Info";
pub(super) const SECTION_FILE_TYPE: &str = "File Type";
pub(super) const SECTION_ANALYSIS: &str = "Analysis";
pub(super) const SECTION_HEX_DUMP: &str = "Hex Dump (first 128 bytes)";
pub(super) const SECTION_PRINTABLE_STRINGS: &str = "Printable Strings";

pub use analysis::calculate_entropy;
pub use hex_dump::create_hex_dump;
pub use strings::extract_printable_strings;

pub fn generate_preview(path: &Path) -> Text<'static> {
    let mut doc = PreviewDoc::new();
    metadata::append_file_metadata(path, &mut doc);

    let Ok(mut file) = fs::File::open(path) else {
        return doc.into_text();
    };

    let header = read_prefix(&mut file, HEADER_READ_BYTES);
    if header.is_empty() {
        return doc.into_text();
    }

    detect::append_file_type_info(path, &header, &mut doc);
    analysis::append_analysis(&mut file, &mut doc);
    hex_dump::append_hex_dump(&header, &mut doc);
    strings::append_printable_strings(path, &mut doc);

    doc.into_text()
}

pub(super) fn read_prefix(file: &mut fs::File, max_bytes: usize) -> Vec<u8> {
    if max_bytes == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; max_bytes];
    let bytes_read = file.read(&mut buffer).unwrap_or(0);
    buffer.truncate(bytes_read);
    buffer
}
