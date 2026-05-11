mod parsers;
mod spotlight;
mod types;
pub(crate) mod ui;

use crate::preview::doc::{format_file_size, format_unix_timestamp};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::fs;
use std::path::Path;

pub use self::types::MediaKind;
use self::types::{MediaMetadata, MediaPreviewPayload};

pub fn detect_media_kind(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?;
    if is_audio_extension(ext) {
        return Some(MediaKind::Audio);
    }
    if is_video_extension(ext) {
        return Some(MediaKind::Video);
    }
    None
}

pub fn generate_preview(path: &Path, kind: MediaKind) -> MediaPreviewPayload {
    let mut lines = Vec::new();

    append_file_info(path, &mut lines);

    let mut metadata = MediaMetadata::default();
    if let Some(spotlight) = spotlight::read_spotlight_metadata(path, &kind) {
        metadata.merge(spotlight);
    }
    if matches!(kind, MediaKind::Audio) {
        if let Some(id3) = parsers::read_mp3_id3(path) {
            metadata.merge(id3);
        }
        if let Some(flac) = parsers::read_flac_metadata(path) {
            metadata.merge(flac);
        }
    }

    append_metadata(path, &kind, &metadata, &mut lines);
    MediaPreviewPayload {
        text: Text::from(lines),
        artwork_bytes: metadata.artwork.as_ref().map(|a| a.data.clone()),
    }
}

fn append_file_info(path: &Path, lines: &mut Vec<Line<'static>>) {
    lines.push(styled_line("File Info", Color::Yellow, true));
    if let Ok(meta) = fs::metadata(path) {
        push_field(
            lines,
            "File",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("-"),
        );
        push_field(lines, "Size", &format_file_size(meta.len()));
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                push_field(
                    lines,
                    "Modified",
                    &format_unix_timestamp(duration.as_secs()),
                );
            }
        }
    } else {
        push_field(lines, "File", "Unable to read file metadata");
    }
    lines.push(Line::from(""));
}

fn append_metadata(
    path: &Path,
    kind: &MediaKind,
    m: &MediaMetadata,
    lines: &mut Vec<Line<'static>>,
) {
    lines.push(styled_line("Metadata", Color::Yellow, true));

    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    push_field(lines, "Name", m.title.as_deref().unwrap_or(fallback_name));
    push_field(lines, "Artist", optional_text(&m.artist));

    match kind {
        MediaKind::Audio => {
            push_field(lines, "Album", optional_text(&m.album));
            push_field(lines, "Genre", optional_text(&m.genre));
            push_field(lines, "Composer", optional_text(&m.composer));
            push_field(lines, "Track", optional_text(&m.track));
        }
        MediaKind::Video => {
            push_field(lines, "Resolution", optional_text(&m.resolution));
            push_field(lines, "Codec", optional_text(&m.codec));
            push_field(lines, "Frame Rate", optional_text(&m.frame_rate));
        }
    }

    push_field(lines, "Duration", optional_text(&m.duration));
    push_field(lines, "Bitrate", optional_text(&m.bitrate));
    push_field(lines, "Sample Rate", optional_text(&m.sample_rate));
    push_field(lines, "Channels", optional_text(&m.channels));
    push_field(lines, "Year", optional_text(&m.year));

    lines.push(Line::from(""));
    lines.push(styled_line("Artwork / Images", Color::Yellow, true));
    if let Some(art) = &m.artwork {
        let mut summary = format!("Embedded image ({}, {} bytes)", art.mime, art.size_bytes);
        if let Some((w, h)) = art.dimensions {
            summary = format!("{summary}, {w}x{h}");
        }
        push_field(lines, "Artwork", &summary);
    } else {
        push_field(lines, "Artwork", "Not available");
    }
}

fn push_field(lines: &mut Vec<Line<'static>>, key: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("   {}: ", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]));
}

fn styled_line(text: &'static str, color: Color, bold: bool) -> Line<'static> {
    let mut style = Style::default().fg(color);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Line::from(vec![Span::styled(text, style)])
}

fn optional_text(v: &Option<String>) -> &str {
    v.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Not available")
}

fn is_audio_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" | "opus" | "aiff" | "alac"
    )
}

fn is_video_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4" | "m4v" | "mov" | "mkv" | "webm" | "avi" | "wmv" | "flv" | "mpeg" | "mpg"
    )
}
