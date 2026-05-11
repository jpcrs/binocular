use std::path::Path;
use std::process::Command;

use super::types::{has_any_metadata, MediaKind, MediaMetadata};

pub(crate) fn read_spotlight_metadata(path: &Path, kind: &MediaKind) -> Option<MediaMetadata> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, kind);
        None
    }

    #[cfg(target_os = "macos")]
    {
        let mut m = MediaMetadata::default();
        m.title = mdls(path, "kMDItemTitle");
        m.artist = mdls(path, "kMDItemAuthors").or_else(|| mdls(path, "kMDItemAuthor"));
        m.album = mdls(path, "kMDItemAlbum");
        m.genre = mdls(path, "kMDItemMusicalGenre");
        m.composer = mdls(path, "kMDItemComposer");
        m.duration = mdls(path, "kMDItemDurationSeconds").map(format_duration_seconds_string);
        m.bitrate = mdls(path, "kMDItemAudioBitRate")
            .or_else(|| mdls(path, "kMDItemTotalBitRate"))
            .map(|v| format_kbps(&v));
        m.sample_rate = mdls(path, "kMDItemAudioSampleRate").map(|v| format!("{v} Hz"));
        m.channels = mdls(path, "kMDItemAudioChannelCount").map(|v| format!("{v} ch"));

        if matches!(kind, MediaKind::Video) {
            let width = mdls(path, "kMDItemPixelWidth");
            let height = mdls(path, "kMDItemPixelHeight");
            if let (Some(w), Some(h)) = (width, height) {
                m.resolution = Some(format!("{w}x{h}"));
            }
            m.codec = mdls(path, "kMDItemCodecs");
        }

        if has_any_metadata(&m) {
            Some(m)
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn mdls(path: &Path, attr: &str) -> Option<String> {
    let output = Command::new("mdls")
        .args(["-raw", "-name", attr, path.to_string_lossy().as_ref()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() || raw == "(null)" {
        return None;
    }

    Some(clean_mdls_value(&raw))
}

#[cfg(target_os = "macos")]
fn clean_mdls_value(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let parts: Vec<String> = inner
            .lines()
            .map(|line| {
                line.trim()
                    .trim_end_matches(',')
                    .trim_matches('"')
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        return parts.join(", ");
    }

    trimmed.trim_matches('"').to_string()
}

fn format_kbps(v: &str) -> String {
    if let Ok(n) = v.parse::<u64>() {
        format!("{} kbps", n / 1000)
    } else {
        v.to_string()
    }
}

fn format_duration_seconds_string(v: String) -> String {
    let secs = v.parse::<f64>().unwrap_or(0.0);
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
