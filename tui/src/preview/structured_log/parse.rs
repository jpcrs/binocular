use crate::preview::structured_log::types::{LogEntry, LogFormat};
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_initial(
    path: &Path,
    format: &LogFormat,
    max_entries: usize,
) -> (Vec<LogEntry>, usize, Vec<String>) {
    let Ok(file) = std::fs::File::open(path) else {
        return (Vec::new(), 0, Vec::new());
    };
    let reader = BufReader::new(file);

    let mut entries: Vec<LogEntry> = Vec::with_capacity(1024);
    let mut total_lines = 0usize;
    let mut field_order: Vec<String> = Vec::new();
    let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        total_lines += 1;

        if entries.len() >= max_entries {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(entry) = parse_line(trimmed, format) {
            for (k, _) in &entry.fields {
                if seen_fields.insert(k.clone()) {
                    field_order.push(k.clone());
                }
            }
            entries.push(entry);
        }
    }

    let all_fields = prioritised_fields(field_order);
    (entries, total_lines, all_fields)
}

pub fn parse_line(line: &str, format: &LogFormat) -> Option<LogEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    match format {
        LogFormat::Jsonl => parse_jsonl(trimmed),
        LogFormat::Logfmt => parse_logfmt(trimmed),
    }
}

fn parse_jsonl(line: &str) -> Option<LogEntry> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;
    let obj = val.as_object()?;
    let fields: Vec<(String, String)> = obj
        .iter()
        .map(|(k, v)| (k.clone(), json_value_to_string(v)))
        .collect();
    Some(LogEntry {
        fields,
        raw: line.to_string(),
    })
}

fn parse_logfmt(line: &str) -> Option<LogEntry> {
    let mut fields = Vec::new();
    let mut rest = line;

    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        let eq = rest.find('=')?;
        let key = rest[..eq].trim().to_string();
        rest = &rest[eq + 1..];

        let value = if rest.starts_with('"') {
            let mut chars = rest[1..].char_indices();
            let mut end = rest.len() - 1;
            let mut prev_backslash = false;
            for (i, c) in chars.by_ref() {
                if c == '"' && !prev_backslash {
                    end = i;
                    break;
                }
                prev_backslash = c == '\\';
            }
            let v = rest[1..end].replace("\\\"", "\"");
            rest = rest.get(end + 2..).unwrap_or("").trim_start_matches(' ');
            v
        } else {
            let end = rest.find(' ').unwrap_or(rest.len());
            let v = rest[..end].to_string();
            rest = rest.get(end..).unwrap_or("").trim_start_matches(' ');
            v
        };

        if !key.is_empty() {
            fields.push((key, value));
        }
    }

    if fields.is_empty() {
        return None;
    }
    Some(LogEntry {
        fields,
        raw: line.to_string(),
    })
}

fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

fn prioritised_fields(mut fields: Vec<String>) -> Vec<String> {
    const PRIORITY: &[&str] = &[
        "time",
        "timestamp",
        "ts",
        "datetime",
        "date",
        "@timestamp",
        "level",
        "severity",
        "lvl",
        "log_level",
        "loglevel",
        "msg",
        "message",
        "text",
        "body",
        "service",
        "app",
        "application",
        "component",
        "error",
        "err",
        "caller",
        "file",
        "line",
    ];

    fields.sort_by_key(|f| {
        let lower = f.to_ascii_lowercase();
        let pos = PRIORITY.iter().position(|&p| p == lower.as_str());
        (pos.unwrap_or(usize::MAX), f.clone())
    });
    fields
}
