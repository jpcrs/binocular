pub mod actions;
mod detect;
mod filter;
mod format;
mod parse;
pub mod reducer;
mod types;
pub(crate) mod ui;
pub mod watcher;

use crate::app::{App, AppAction};
use crate::preview::structured_log::actions::LogViewerOutcome;
use crate::preview::{PreviewContent, PreviewSource};
use crossterm::event::KeyEvent;

pub const DEFAULT_MAX_ENTRIES: usize = 100_000;
pub const STDIN_STREAM_PATH: &str = "<stdin>";
pub use detect::detect_structured_log;
pub use filter::{parse_epoch_secs, parse_filters};
pub use format::format_entry_visible;
pub use parse::{parse_initial, parse_line};
pub use types::{
    ColModal, ColumnConfig, FilterOp, LogEntry, LogFilter, LogFilterState, LogFormat, StructuredLog,
};

pub fn preview_content(log: StructuredLog) -> PreviewContent {
    reducer::preview_content(log)
}

pub fn initialize_empty_stream(app: &mut App, path: String, format: LogFormat) {
    app.preview_session.preview.source = Some(PreviewSource::LogStream(path));
    app.preview_session.preview.content = Some(preview_content(StructuredLog {
        entries: vec![],
        total_lines: 0,
        all_fields: vec![],
        format,
    }));
}

pub fn handle_input(app: &mut App, key: KeyEvent) {
    let Some(PreviewContent::StructuredLog(lp)) = &mut app.preview_session.preview.content else {
        return;
    };

    let Some(action) = actions::action_for_key(lp, key) else {
        return;
    };

    match reducer::apply_action(lp, action, app.runtime.run.log) {
        LogViewerOutcome::None => {}
        LogViewerOutcome::ExitApp => app.apply_action(AppAction::Quit),
        LogViewerOutcome::FocusSearch => {
            app.ui.layout.preview_fullscreen = false;
            app.apply_action(AppAction::FocusSearch);
        }
    }
}

pub fn apply_append(app: &mut App, path: &str, entries: Vec<LogEntry>, max_entries: usize) {
    let Some(PreviewSource::LogStream(active_path)) = app.preview_session.preview.source.as_ref()
    else {
        return;
    };

    let is_valid_source = path == active_path
        || app
            .runtime
            .run
            .log_files
            .iter()
            .any(|f| f.to_str() == Some(path));
    if !is_valid_source {
        return;
    }

    let Some(PreviewContent::StructuredLog(lp)) = &mut app.preview_session.preview.content else {
        return;
    };

    reducer::append_entries(lp, entries, max_entries);
}

pub fn init_visible_cols(fields: &[String], entries: &[LogEntry]) -> Vec<ColumnConfig> {
    const WIDTH_SAMPLE: usize = 200;
    const MAX_COL_WIDTH: usize = 40;
    const MIN_COL_WIDTH: usize = 6;
    const MSG_FIELDS: &[&str] = &["msg", "message", "text", "body", "log"];

    let mut widths: Vec<usize> = fields
        .iter()
        .map(|f| f.chars().count().max(MIN_COL_WIDTH))
        .collect();

    for entry in entries.iter().take(WIDTH_SAMPLE) {
        for (col_i, field) in fields.iter().enumerate() {
            if let Some((_, v)) = entry.fields.iter().find(|(k, _)| k == field) {
                let len = v.chars().count().min(MAX_COL_WIDTH);
                if len > widths[col_i] {
                    widths[col_i] = len;
                }
            }
        }
    }

    for w in &mut widths {
        *w = (*w).min(MAX_COL_WIDTH);
    }

    for (i, field) in fields.iter().enumerate() {
        let lower = field.to_ascii_lowercase();
        if MSG_FIELDS.iter().any(|m| *m == lower.as_str()) {
            widths[i] = widths[i].max(40).min(60);
        }
    }

    fields
        .iter()
        .zip(widths.iter())
        .map(|(f, &w)| ColumnConfig {
            field: f.clone(),
            width: w,
        })
        .collect()
}

pub fn update_fields(all_fields: &mut Vec<String>, entry: &LogEntry) {
    for (k, _) in &entry.fields {
        if !all_fields.iter().any(|f| f == k) {
            all_fields.push(k.clone());
        }
    }
}
