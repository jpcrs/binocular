use crate::preview::structured_log::types::{
    ColModal, ColumnConfig, FilterOp, LogEntry, LogFilter, LogFilterState, StructuredLog,
};

const TIMESTAMP_FIELD_NAMES: &[&str] =
    &["time", "timestamp", "ts", "datetime", "date", "@timestamp"];

impl LogFilterState {
    pub fn recompute_matches(&mut self, log: &StructuredLog) {
        self.cached_matches = if self.filters.is_empty() {
            (0..log.entries.len()).rev().collect()
        } else {
            log.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| entry_matches(e, &self.filters))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        };
        self.clamp_cursor();
    }

    pub fn extend_matches(&mut self, log: &StructuredLog, from_index: usize) {
        let mut new: Vec<usize> = (from_index..log.entries.len())
            .filter(|&i| self.filters.is_empty() || entry_matches(&log.entries[i], &self.filters))
            .collect();
        new.reverse();
        let old = std::mem::take(&mut self.cached_matches);
        self.cached_matches = new;
        self.cached_matches.extend(old);
    }

    pub fn apply_input(&mut self, log: &StructuredLog) {
        self.filters = parse_filters(&self.input);
        self.cursor = 0;
        self.scroll = 0;
        self.recompute_matches(log);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.cached_matches.len().saturating_sub(1);
        self.cursor = (self.cursor + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.cursor = self.cached_matches.len().saturating_sub(1);
    }

    pub fn move_col_left(&mut self) {
        self.selected_col = self.selected_col.saturating_sub(1);
    }

    pub fn move_col_right(&mut self) {
        if !self.visible_cols.is_empty() {
            self.selected_col = (self.selected_col + 1).min(self.visible_cols.len() - 1);
        }
    }

    pub fn hide_selected_col(&mut self) {
        if self.visible_cols.len() <= 1 {
            return;
        }
        self.visible_cols.remove(self.selected_col);
        self.selected_col = self
            .selected_col
            .min(self.visible_cols.len().saturating_sub(1));
    }

    pub fn isolate_selected_col(&mut self) {
        if self.visible_cols.is_empty() {
            return;
        }
        let kept = self.visible_cols.remove(self.selected_col);
        self.visible_cols = vec![kept];
        self.selected_col = 0;
        self.col_scroll = 0;
    }

    pub fn resize_selected_col(&mut self, delta: i32) {
        if let Some(col) = self.visible_cols.get_mut(self.selected_col) {
            col.width = ((col.width as i32 + delta).max(3)) as usize;
        }
    }

    pub fn open_col_modal(&mut self, all_fields: &[String]) {
        let visible_set: std::collections::HashSet<&str> =
            self.visible_cols.iter().map(|c| c.field.as_str()).collect();
        let checked = all_fields
            .iter()
            .map(|f| visible_set.contains(f.as_str()))
            .collect();
        self.col_modal = Some(ColModal {
            cursor: 0,
            checked,
            scroll: 0,
        });
    }

    pub fn apply_modal_changes(&mut self, all_fields: &[String]) {
        let Some(modal) = self.col_modal.take() else {
            return;
        };

        let current_widths: std::collections::HashMap<&str, usize> = self
            .visible_cols
            .iter()
            .map(|c| (c.field.as_str(), c.width))
            .collect();

        let new_cols: Vec<ColumnConfig> = all_fields
            .iter()
            .enumerate()
            .filter(|(i, _)| modal.checked.get(*i).copied().unwrap_or(false))
            .map(|(_, f)| ColumnConfig {
                field: f.clone(),
                width: current_widths.get(f.as_str()).copied().unwrap_or(15),
            })
            .collect();

        if !new_cols.is_empty() {
            self.visible_cols = new_cols;
            self.selected_col = self
                .selected_col
                .min(self.visible_cols.len().saturating_sub(1));
            self.col_scroll = self.col_scroll.min(self.selected_col);
        }
    }

    pub fn add_new_visible_col(&mut self, field: &str) {
        if !self.visible_cols.iter().any(|c| c.field == field) {
            self.visible_cols.push(ColumnConfig {
                field: field.to_string(),
                width: 15,
            });
        }
    }

    pub fn toggle_mark(&mut self) {
        let Some(&entry_idx) = self.cached_matches.get(self.cursor) else {
            return;
        };
        if !self.marked.remove(&entry_idx) {
            self.marked.insert(entry_idx);
        }
    }

    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    fn clamp_cursor(&mut self) {
        let max = self.cached_matches.len().saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }
        if self.scroll > self.cursor {
            self.scroll = self.cursor;
        }
    }
}

fn entry_matches(entry: &LogEntry, filters: &[LogFilter]) -> bool {
    filters.iter().all(|f| {
        if let FilterOp::Since(cutoff) = &f.op {
            let ts_val = entry
                .fields
                .iter()
                .find(|(k, _)| {
                    let lower = k.to_ascii_lowercase();
                    TIMESTAMP_FIELD_NAMES.iter().any(|c| *c == lower.as_str())
                })
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            return parse_epoch_secs(ts_val)
                .map(|e| e >= *cutoff)
                .unwrap_or(false);
        }

        let needle = f.value.to_lowercase();
        match &f.field {
            None => entry
                .fields
                .iter()
                .any(|(_, v)| match_value(v, &needle, &f.op)),
            Some(fname) => {
                let val = entry
                    .fields
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(fname))
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                match_value(val, &needle, &f.op)
            }
        }
    })
}

fn match_value(haystack: &str, needle: &str, op: &FilterOp) -> bool {
    match op {
        FilterOp::Contains => haystack.to_lowercase().contains(needle),
        FilterOp::Equals => haystack.eq_ignore_ascii_case(needle),
        FilterOp::NotEquals => !haystack.eq_ignore_ascii_case(needle),
        FilterOp::Since(_) => true,
    }
}

pub fn parse_filters(input: &str) -> Vec<LogFilter> {
    input
        .split_whitespace()
        .filter_map(|tok| {
            if let Some(arg) = tok.strip_prefix("last:") {
                return parse_time_cutoff(arg).map(|cutoff| LogFilter {
                    field: None,
                    op: FilterOp::Since(cutoff),
                    value: tok.to_string(),
                });
            }

            Some(if let Some(idx) = tok.find("!=") {
                LogFilter {
                    field: Some(tok[..idx].to_string()),
                    op: FilterOp::NotEquals,
                    value: tok[idx + 2..].to_string(),
                }
            } else if let Some(idx) = tok.find('=') {
                LogFilter {
                    field: Some(tok[..idx].to_string()),
                    op: FilterOp::Equals,
                    value: tok[idx + 1..].to_string(),
                }
            } else if let Some(idx) = tok.find(':') {
                if idx > 0 {
                    LogFilter {
                        field: Some(tok[..idx].to_string()),
                        op: FilterOp::Contains,
                        value: tok[idx + 1..].to_string(),
                    }
                } else {
                    LogFilter {
                        field: None,
                        op: FilterOp::Contains,
                        value: tok[1..].to_string(),
                    }
                }
            } else {
                LogFilter {
                    field: None,
                    op: FilterOp::Contains,
                    value: tok.to_string(),
                }
            })
        })
        .collect()
}

pub fn parse_epoch_secs(val: &str) -> Option<u64> {
    let trimmed = val.trim();
    if let Ok(n) = trimmed.parse::<u64>() {
        return Some(if n > 10_000_000_000 { n / 1000 } else { n });
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        if f > 0.0 {
            return Some(if f > 1e10 {
                (f / 1000.0) as u64
            } else {
                f as u64
            });
        }
    }
    parse_datetime_to_epoch(trimmed)
}

fn parse_time_cutoff(arg: &str) -> Option<u64> {
    let (num_str, secs_per_unit) = if let Some(n) = arg.strip_suffix('h') {
        (n, 3600u64)
    } else if let Some(n) = arg.strip_suffix('m') {
        (n, 60u64)
    } else if let Some(n) = arg.strip_suffix('d') {
        (n, 86_400u64)
    } else if let Some(n) = arg.strip_suffix('s') {
        (n, 1u64)
    } else {
        return None;
    };
    let n: u64 = num_str.parse().ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(now.saturating_sub(n * secs_per_unit))
}

fn parse_datetime_to_epoch(s: &str) -> Option<u64> {
    let s = s.trim_end_matches('Z');
    let sep = s.find('T').or_else(|| s.find(' '))?;
    let date_str = &s[..sep];
    let time_str = &s[sep + 1..];

    let mut dp = date_str.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: i64 = dp.next()?.parse().ok()?;
    let day: i64 = dp.next()?.parse().ok()?;

    let time_str = time_str.split('+').next().unwrap_or(time_str);
    let time_str = if time_str.len() > 8 {
        if let Some(pos) = time_str[8..].find('-') {
            &time_str[..8 + pos]
        } else {
            time_str
        }
    } else {
        time_str
    };

    let mut tp = time_str.split(':');
    let hour: i64 = tp.next()?.parse().ok()?;
    let min: i64 = tp.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let sec: i64 = tp
        .next()
        .and_then(|s| s.split('.').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let epoch_days = days_from_civil(year, month, day);
    let epoch_secs = epoch_days * 86400 + hour * 3600 + min * 60 + sec;
    if epoch_secs >= 0 {
        Some(epoch_secs as u64)
    } else {
        None
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
