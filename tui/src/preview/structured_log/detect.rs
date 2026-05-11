//! Structured-log file detection.

use crate::preview::structured_log::types::LogFormat;
use std::io::{BufRead, BufReader};
use std::path::Path;

const DETECT_SAMPLE: usize = 20;
const DETECT_THRESHOLD: f32 = 0.75;

pub fn detect_structured_log(path: &Path) -> Option<LogFormat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "jsonl" | "ndjson" => return Some(LogFormat::Jsonl),
        "log" | "logs" => {}
        _ => return None,
    }

    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .take(DETECT_SAMPLE)
        .filter_map(|l| l.ok())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return None;
    }

    let json_hits = lines
        .iter()
        .filter(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .map(|v| v.is_object())
                .unwrap_or(false)
        })
        .count();
    if json_hits as f32 / lines.len() as f32 >= DETECT_THRESHOLD {
        return Some(LogFormat::Jsonl);
    }

    let logfmt_hits = lines.iter().filter(|l| looks_like_logfmt(l)).count();
    if logfmt_hits as f32 / lines.len() as f32 >= DETECT_THRESHOLD {
        return Some(LogFormat::Logfmt);
    }

    None
}

fn looks_like_logfmt(line: &str) -> bool {
    line.split_whitespace()
        .any(|t| t.contains('=') && !t.starts_with('=') && !t.ends_with('='))
}
