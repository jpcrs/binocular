use crate::preview::structured_log::types::{ColumnConfig, LogEntry};

pub fn format_entry_visible(entry: &LogEntry, visible_cols: &[ColumnConfig]) -> String {
    visible_cols
        .iter()
        .filter_map(|col| {
            entry
                .fields
                .iter()
                .find(|(k, _)| k == &col.field)
                .map(|(k, v)| {
                    if v.contains(' ') || v.contains('"') || v.is_empty() {
                        format!("{}=\"{}\"", k, v.replace('"', "\\\""))
                    } else {
                        format!("{}={}", k, v)
                    }
                })
        })
        .collect::<Vec<_>>()
        .join("  ")
}
