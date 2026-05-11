use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::types::{has_any_metadata, merge_opt, ArtworkInfo, MediaMetadata};

/// Maximum ID3 tag size to prevent memory exhaustion from malformed headers (20 MiB).
const MAX_ID3_TAG_SIZE: usize = 20 * 1024 * 1024;

pub(crate) fn read_mp3_id3(path: &Path) -> Option<MediaMetadata> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    if !ext.eq_ignore_ascii_case("mp3") {
        return None;
    }

    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 10];
    file.read_exact(&mut header).ok()?;
    if &header[0..3] != b"ID3" {
        return None;
    }

    let version_major = header[3];
    if version_major != 3 && version_major != 4 {
        return None;
    }

    let tag_size = synchsafe_to_u32(&header[6..10]) as usize;
    if tag_size == 0 || tag_size > MAX_ID3_TAG_SIZE {
        return None;
    }

    let mut buf = vec![0u8; tag_size];
    file.seek(SeekFrom::Start(10)).ok()?;
    file.read_exact(&mut buf).ok()?;

    let mut meta = MediaMetadata::default();
    let mut offset = 0usize;
    while offset + 10 <= buf.len() {
        let frame_id = &buf[offset..offset + 4];
        if frame_id.iter().all(|b| *b == 0) {
            break;
        }

        let frame_size = if version_major == 4 {
            synchsafe_to_u32(&buf[offset + 4..offset + 8]) as usize
        } else {
            u32::from_be_bytes([
                buf[offset + 4],
                buf[offset + 5],
                buf[offset + 6],
                buf[offset + 7],
            ]) as usize
        };

        if frame_size == 0 || offset + 10 + frame_size > buf.len() {
            break;
        }

        let data = &buf[offset + 10..offset + 10 + frame_size];
        let id = std::str::from_utf8(frame_id).unwrap_or("");
        match id {
            "TIT2" => meta.title = parse_text_frame(data),
            "TPE1" => meta.artist = parse_text_frame(data),
            "TALB" => meta.album = parse_text_frame(data),
            "TCON" => meta.genre = parse_text_frame(data),
            "TCOM" => meta.composer = parse_text_frame(data),
            "TRCK" => meta.track = parse_text_frame(data),
            "TYER" | "TDRC" => meta.year = parse_text_frame(data),
            "APIC" => meta.artwork = parse_apic_frame(data),
            _ => {}
        }

        offset += 10 + frame_size;
    }

    if has_any_metadata(&meta) {
        Some(meta)
    } else {
        None
    }
}

pub(crate) fn read_flac_metadata(path: &Path) -> Option<MediaMetadata> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    if !ext.eq_ignore_ascii_case("flac") {
        return None;
    }

    let mut file = fs::File::open(path).ok()?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).ok()?;
    if &magic != b"fLaC" {
        return None;
    }

    let mut meta = MediaMetadata::default();
    let file_size = fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0);

    let mut last = false;
    while !last {
        let mut hdr = [0u8; 4];
        file.read_exact(&mut hdr).ok()?;
        last = (hdr[0] & 0x80) != 0;
        let block_type = hdr[0] & 0x7F;
        let len = ((hdr[1] as usize) << 16) | ((hdr[2] as usize) << 8) | (hdr[3] as usize);

        let mut data = vec![0u8; len];
        file.read_exact(&mut data).ok()?;

        match block_type {
            0 => parse_flac_streaminfo(&data, file_size, &mut meta),
            4 => parse_flac_vorbis_comment(&data, &mut meta),
            6 => {
                if meta.artwork.is_none() {
                    meta.artwork = parse_flac_picture(&data);
                }
            }
            _ => {}
        }
    }

    if has_any_metadata(&meta) {
        Some(meta)
    } else {
        None
    }
}

fn parse_flac_streaminfo(data: &[u8], file_size: u64, meta: &mut MediaMetadata) {
    if data.len() < 34 {
        return;
    }

    let packed = &data[10..18];
    let v = u64::from_be_bytes([
        packed[0], packed[1], packed[2], packed[3], packed[4], packed[5], packed[6], packed[7],
    ]);

    let sample_rate = ((v >> 44) & 0xFFFFF) as u32;
    let channels = ((v >> 41) & 0x7) as u32 + 1;
    let total_samples = v & 0xFFFFFFFFF;

    if sample_rate > 0 {
        meta.sample_rate = Some(format!("{sample_rate} Hz"));
        meta.channels = Some(format!("{channels} ch"));

        let duration_secs = total_samples as f64 / sample_rate as f64;
        if duration_secs > 0.0 {
            meta.duration = Some(format_duration_seconds(duration_secs));
            if file_size > 0 {
                let bps = ((file_size as f64 * 8.0) / duration_secs) as u64;
                meta.bitrate = Some(format!("{} kbps", bps / 1000));
            }
        }
    }
}

fn parse_flac_vorbis_comment(data: &[u8], meta: &mut MediaMetadata) {
    let mut idx = 0usize;

    let Some(vendor_len) = read_u32_le(data, &mut idx) else {
        return;
    };
    idx = idx.saturating_add(vendor_len as usize);
    if idx > data.len() {
        return;
    }

    let Some(count) = read_u32_le(data, &mut idx) else {
        return;
    };

    for _ in 0..count {
        let Some(len) = read_u32_le(data, &mut idx) else {
            return;
        };
        let len = len as usize;
        if idx + len > data.len() {
            return;
        }
        let entry = String::from_utf8_lossy(&data[idx..idx + len]).to_string();
        idx += len;

        let Some((k, v)) = entry.split_once('=') else {
            continue;
        };
        let value = v.trim().to_string();
        if value.is_empty() {
            continue;
        }

        match k.to_ascii_uppercase().as_str() {
            "TITLE" => merge_opt(&mut meta.title, Some(value)),
            "ARTIST" => merge_opt(&mut meta.artist, Some(value)),
            "ALBUM" => merge_opt(&mut meta.album, Some(value)),
            "GENRE" => merge_opt(&mut meta.genre, Some(value)),
            "COMPOSER" => merge_opt(&mut meta.composer, Some(value)),
            "TRACKNUMBER" | "TRACK" => merge_opt(&mut meta.track, Some(value)),
            "DATE" | "YEAR" => merge_opt(&mut meta.year, Some(value)),
            _ => {}
        }
    }
}

fn parse_flac_picture(data: &[u8]) -> Option<ArtworkInfo> {
    let mut idx = 0usize;
    let _pic_type = read_u32_be(data, &mut idx)?;
    let mime_len = read_u32_be(data, &mut idx)? as usize;
    if idx + mime_len > data.len() {
        return None;
    }
    let mime = String::from_utf8_lossy(&data[idx..idx + mime_len]).to_string();
    idx += mime_len;

    let desc_len = read_u32_be(data, &mut idx)? as usize;
    if idx + desc_len > data.len() {
        return None;
    }
    idx += desc_len;

    let width = read_u32_be(data, &mut idx)?;
    let height = read_u32_be(data, &mut idx)?;
    let _depth = read_u32_be(data, &mut idx)?;
    let _colors = read_u32_be(data, &mut idx)?;
    let data_len = read_u32_be(data, &mut idx)? as usize;
    if idx + data_len > data.len() {
        return None;
    }
    let image_bytes = &data[idx..idx + data_len];

    let dimensions = if width > 0 && height > 0 {
        Some((width, height))
    } else {
        image::load_from_memory(image_bytes)
            .ok()
            .map(|img| (img.width(), img.height()))
    };

    Some(ArtworkInfo {
        mime: if mime.is_empty() {
            "unknown".to_string()
        } else {
            mime
        },
        size_bytes: data_len,
        dimensions,
        data: image_bytes.to_vec(),
    })
}

fn read_u32_le(data: &[u8], idx: &mut usize) -> Option<u32> {
    if *idx + 4 > data.len() {
        return None;
    }
    let out = u32::from_le_bytes([data[*idx], data[*idx + 1], data[*idx + 2], data[*idx + 3]]);
    *idx += 4;
    Some(out)
}

fn read_u32_be(data: &[u8], idx: &mut usize) -> Option<u32> {
    if *idx + 4 > data.len() {
        return None;
    }
    let out = u32::from_be_bytes([data[*idx], data[*idx + 1], data[*idx + 2], data[*idx + 3]]);
    *idx += 4;
    Some(out)
}

fn parse_text_frame(data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let encoding = data[0];
    let text_bytes = &data[1..];
    let text = decode_text_with_encoding(encoding, text_bytes)?;
    let trimmed = text.trim_matches('\0').trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_apic_frame(data: &[u8]) -> Option<ArtworkInfo> {
    if data.len() < 4 {
        return None;
    }
    let encoding = data[0];
    let mut idx = 1usize;

    let mime_end = data[idx..].iter().position(|b| *b == 0)?;
    let mime = String::from_utf8_lossy(&data[idx..idx + mime_end]).to_string();
    idx += mime_end + 1;
    if idx >= data.len() {
        return None;
    }

    idx += 1;
    if idx >= data.len() {
        return None;
    }

    let desc_end = find_encoded_terminator(&data[idx..], encoding)?;
    idx += desc_end;
    idx += if encoding == 1 || encoding == 2 { 2 } else { 1 };
    if idx >= data.len() {
        return None;
    }

    let image_bytes = &data[idx..];
    let dimensions = image::load_from_memory(image_bytes)
        .ok()
        .map(|img| (img.width(), img.height()));

    Some(ArtworkInfo {
        mime: if mime.is_empty() {
            "unknown".to_string()
        } else {
            mime
        },
        size_bytes: image_bytes.len(),
        dimensions,
        data: image_bytes.to_vec(),
    })
}

fn find_encoded_terminator(data: &[u8], encoding: u8) -> Option<usize> {
    if encoding == 1 || encoding == 2 {
        let mut i = 0usize;
        while i + 1 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 {
                return Some(i);
            }
            i += 2;
        }
        None
    } else {
        data.iter().position(|b| *b == 0)
    }
}

fn decode_text_with_encoding(encoding: u8, bytes: &[u8]) -> Option<String> {
    match encoding {
        0 => Some(bytes.iter().map(|b| *b as char).collect()),
        3 => Some(String::from_utf8_lossy(bytes).to_string()),
        1 => decode_utf16_with_optional_bom(bytes),
        2 => decode_utf16_be(bytes),
        _ => None,
    }
}

fn decode_utf16_with_optional_bom(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 {
        return None;
    }

    if bytes[0] == 0xFF && bytes[1] == 0xFE {
        return decode_utf16_endian(&bytes[2..], true);
    }
    if bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16_endian(&bytes[2..], false);
    }

    decode_utf16_endian(bytes, false)
}

fn decode_utf16_be(bytes: &[u8]) -> Option<String> {
    decode_utf16_endian(bytes, false)
}

fn decode_utf16_endian(bytes: &[u8], little: bool) -> Option<String> {
    if bytes.len() < 2 {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if little {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();

    Some(String::from_utf16_lossy(&units))
}

fn synchsafe_to_u32(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 21)
        | ((bytes[1] as u32) << 14)
        | ((bytes[2] as u32) << 7)
        | (bytes[3] as u32)
}

fn format_duration_seconds(secs: f64) -> String {
    let total = secs.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}
