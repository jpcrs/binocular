//! Binary type detection and format-specific metadata.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const MAX_ZIP_ENTRIES_TO_SCAN: usize = 10_000;

pub fn append_file_type_info(
    path: &Path,
    header: &[u8],
    doc: &mut crate::preview::doc::PreviewDoc,
) {
    use ratatui::style::Color;

    doc.push_section(super::SECTION_FILE_TYPE);

    let (file_type, description) = detect_file_type(header);
    doc.push_field("Type", file_type, Color::Green);
    if !description.is_empty() {
        doc.push_field("Description", description, Color::White);
    }

    if let Some(extra_info) = get_format_specific_info(header, path) {
        for (label, value) in extra_info {
            doc.push_field(label, value, Color::White);
        }
    }

    doc.push_blank_line();
}

fn detect_file_type(header: &[u8]) -> (&'static str, &'static str) {
    if header.len() < 4 {
        return ("Unknown", "");
    }

    match &header[..4.min(header.len())] {
        [0x7f, b'E', b'L', b'F'] => ("ELF", "Executable and Linkable Format (Linux/Unix)"),
        [b'M', b'Z', ..] => ("PE/DOS", "Windows Executable"),
        [0xfe, 0xed, 0xfa, 0xce] | [0xce, 0xfa, 0xed, 0xfe] => {
            ("Mach-O", "macOS Executable (32-bit)")
        }
        [0xfe, 0xed, 0xfa, 0xcf] | [0xcf, 0xfa, 0xed, 0xfe] => {
            ("Mach-O", "macOS Executable (64-bit)")
        }
        [0xca, 0xfe, 0xba, 0xbe] => ("Mach-O", "macOS Universal Binary"),
        [b'P', b'K', 0x03, 0x04] => ("ZIP", "ZIP Archive"),
        [b'P', b'K', 0x05, 0x06] => ("ZIP", "ZIP Archive (empty)"),
        [0x1f, 0x8b, ..] => ("GZIP", "GZIP Compressed"),
        [b'B', b'Z', b'h', ..] => ("BZIP2", "BZIP2 Compressed"),
        [0xfd, b'7', b'z', b'X'] => ("XZ", "XZ Compressed"),
        [b'R', b'a', b'r', b'!'] => ("RAR", "RAR Archive"),
        [b'7', b'z', 0xbc, 0xaf] => ("7Z", "7-Zip Archive"),
        [0x89, b'P', b'N', b'G'] => ("PNG", "Portable Network Graphics"),
        [0xff, 0xd8, 0xff, ..] => ("JPEG", "JPEG Image"),
        [b'G', b'I', b'F', b'8'] => ("GIF", "Graphics Interchange Format"),
        [b'B', b'M', ..] => ("BMP", "Bitmap Image"),
        [b'R', b'I', b'F', b'F'] if header.len() >= 12 && &header[8..12] == b"WEBP" => {
            ("WEBP", "WebP Image")
        }
        [b'I', b'I', 0x2a, 0x00] | [b'M', b'M', 0x00, 0x2a] => ("TIFF", "Tagged Image File Format"),
        [0x25, b'P', b'D', b'F'] => ("PDF", "Portable Document Format"),
        [0xd0, 0xcf, 0x11, 0xe0] => ("OLE", "Microsoft Office Document (legacy)"),
        [b'I', b'D', b'3', ..] => ("MP3", "MP3 Audio (ID3 tagged)"),
        [0xff, 0xfb, ..] | [0xff, 0xfa, ..] => ("MP3", "MP3 Audio"),
        [b'O', b'g', b'g', b'S'] => ("OGG", "Ogg Container"),
        [b'f', b'L', b'a', b'C'] => ("FLAC", "Free Lossless Audio Codec"),
        [b'R', b'I', b'F', b'F'] if header.len() >= 12 && &header[8..12] == b"WAVE" => {
            ("WAV", "Waveform Audio")
        }
        [b'R', b'I', b'F', b'F'] if header.len() >= 12 && &header[8..12] == b"AVI " => {
            ("AVI", "Audio Video Interleave")
        }
        [b'S', b'Q', b'L', b'i'] => ("SQLite", "SQLite Database"),
        [0x00, 0x00, 0x00, ..] if header.len() >= 8 && &header[4..8] == b"ftyp" => {
            ("MP4/MOV", "MPEG-4 / QuickTime")
        }
        [0x1a, 0x45, 0xdf, 0xa3] => ("MKV/WebM", "Matroska/WebM Container"),
        _ => ("Binary", "Unknown binary format"),
    }
}

fn get_format_specific_info(header: &[u8], path: &Path) -> Option<Vec<(&'static str, String)>> {
    if header.len() < 4 {
        return None;
    }

    let mut info = Vec::new();

    if let Some(entries) = elf_info(header) {
        info.extend(entries);
    }

    if let Some((width, height)) = png_dimensions(header) {
        info.push(("Dimensions", format!("{}x{}", width, height)));
    }

    if header.starts_with(&[0xff, 0xd8, 0xff]) {
        if let Some((w, h)) = find_jpeg_dimensions(path) {
            info.push(("Dimensions", format!("{}x{}", w, h)));
        }
    }

    if header.starts_with(&[b'P', b'K', 0x03, 0x04]) {
        if let Some(count) = count_zip_entries(path) {
            info.push(("Entries", count.to_string()));
        }
    }

    if info.is_empty() {
        None
    } else {
        Some(info)
    }
}

fn elf_info(header: &[u8]) -> Option<Vec<(&'static str, String)>> {
    if !header.starts_with(&[0x7f, b'E', b'L', b'F']) || header.len() < 20 {
        return None;
    }

    let class = match header.get(4) {
        Some(1) => "32-bit",
        Some(2) => "64-bit",
        _ => "Unknown",
    };
    let endian = match header.get(5) {
        Some(1) => "Little-endian",
        Some(2) => "Big-endian",
        _ => "Unknown",
    };
    let os_abi = match header.get(7) {
        Some(0) => "System V",
        Some(3) => "Linux",
        Some(9) => "FreeBSD",
        Some(12) => "OpenBSD",
        _ => "Other",
    };

    Some(vec![
        ("Class", class.to_string()),
        ("Endianness", endian.to_string()),
        ("OS/ABI", os_abi.to_string()),
    ])
}

fn png_dimensions(header: &[u8]) -> Option<(u32, u32)> {
    if !header.starts_with(&[0x89, b'P', b'N', b'G']) || header.len() < 24 {
        return None;
    }

    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Some((width, height))
}

fn find_jpeg_dimensions(path: &Path) -> Option<(u16, u16)> {
    let mut file = fs::File::open(path).ok()?;
    let mut buffer = [0u8; 12];

    file.seek(SeekFrom::Start(2)).ok()?;

    loop {
        if file.read_exact(&mut buffer[..2]).is_err() {
            break;
        }

        if buffer[0] != 0xff {
            break;
        }

        let marker = buffer[1];

        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if file.read_exact(&mut buffer[..7]).is_err() {
                break;
            }
            let height = u16::from_be_bytes([buffer[3], buffer[4]]);
            let width = u16::from_be_bytes([buffer[5], buffer[6]]);
            return Some((width, height));
        }

        if file.read_exact(&mut buffer[..2]).is_err() {
            break;
        }
        let length = u16::from_be_bytes([buffer[0], buffer[1]]) as i64 - 2;
        if length > 0 {
            file.seek(SeekFrom::Current(length)).ok()?;
        }
    }

    None
}

fn count_zip_entries(path: &Path) -> Option<usize> {
    let mut file = fs::File::open(path).ok()?;
    let mut count = 0;
    let mut buffer = [0u8; 30];

    loop {
        if file.read_exact(&mut buffer[..4]).is_err() {
            break;
        }

        if &buffer[..4] != &[b'P', b'K', 0x03, 0x04] {
            break;
        }

        count += 1;
        if count > MAX_ZIP_ENTRIES_TO_SCAN {
            break;
        }

        if file.read_exact(&mut buffer[..26]).is_err() {
            break;
        }

        let compressed_size = u32::from_le_bytes([buffer[14], buffer[15], buffer[16], buffer[17]]);
        let filename_len = u16::from_le_bytes([buffer[22], buffer[23]]) as i64;
        let extra_len = u16::from_le_bytes([buffer[24], buffer[25]]) as i64;

        let skip = filename_len + extra_len + compressed_size as i64;
        if file.seek(SeekFrom::Current(skip)).is_err() {
            break;
        }
    }

    Some(count)
}
