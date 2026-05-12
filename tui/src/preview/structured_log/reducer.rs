use super::actions::{
    ColumnAction, CursorAction, FilterAction, LogViewerAction, LogViewerOutcome, ModalAction,
};
use crate::preview::structured_log::{
    format_entry_visible, init_visible_cols, LogEntry, StructuredLog,
};
use crate::preview::types::LogPreview;
use crate::preview::PreviewContent;
pub(crate) fn preview_content(log: StructuredLog) -> PreviewContent {
    let visible_cols = init_visible_cols(&log.all_fields, &log.entries);
    let cached_matches = (0..log.entries.len()).rev().collect();
    let filter_state = crate::preview::structured_log::LogFilterState {
        cached_matches,
        visible_cols,
        ..Default::default()
    };

    PreviewContent::StructuredLog(LogPreview { log, filter_state })
}

pub(crate) fn append_entries(lp: &mut LogPreview, entries: Vec<LogEntry>, max_entries: usize) {
    if lp.filter_state.paused {
        return;
    }

    let from = lp.log.entries.len();

    let mut new_fields: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(32);

    for entry in entries {
        lp.log.total_lines += 1;
        if lp.log.entries.len() >= max_entries {
            continue;
        }

        for (field, _) in &entry.fields {
            if !lp.log.all_fields.iter().any(|f| f == field) && new_fields.insert(field.clone()) {
            {
                lp.filter_state.add_new_visible_col(field);
            }
        }

        lp.log.entries.push(entry);
    }

    for field in &new_fields {
        lp.log.all_fields.push(field.clone());
    }

    lp.filter_state.extend_matches(&lp.log, from);
    if !lp.filter_state.input_active && lp.filter_state.cursor == 0 {
        lp.filter_state.scroll = 0;
    }
}

pub(crate) fn apply_action(
    lp: &mut LogPreview,
    action: LogViewerAction,
    standalone_log_mode: bool,
) -> LogViewerOutcome {
    match action {
        LogViewerAction::Filter(action) => apply_filter_action(lp, action),
        LogViewerAction::Cursor(action) => apply_cursor_action(lp, action),
        LogViewerAction::Column(action) => apply_column_action(lp, action),
        LogViewerAction::Modal(action) => apply_modal_action(lp, action),
        LogViewerAction::TogglePause => lp.filter_state.paused = !lp.filter_state.paused,
        LogViewerAction::ToggleMark => lp.filter_state.toggle_mark(),
        LogViewerAction::Copy { raw } => copy_entries(lp, raw),
        LogViewerAction::ResetView => reset_filter_and_columns(lp),
        LogViewerAction::Exit => {
            return if standalone_log_mode {
                LogViewerOutcome::ExitApp
            } else {
                LogViewerOutcome::FocusSearch
            };
        }
    }

    LogViewerOutcome::None
}

fn reset_filter_and_columns(lp: &mut LogPreview) {
    lp.filter_state.input.clear();
    lp.filter_state.filters.clear();
    lp.filter_state.cursor = 0;
    lp.filter_state.scroll = 0;
    lp.filter_state.recompute_matches(&lp.log);
    lp.filter_state.visible_cols = init_visible_cols(&lp.log.all_fields, &lp.log.entries);
    lp.filter_state.selected_col = 0;
    lp.filter_state.col_scroll = 0;
}

fn apply_filter_action(lp: &mut LogPreview, action: FilterAction) {
    match action {
        FilterAction::StartEditing => lp.filter_state.input_active = true,
        FilterAction::StopEditing => lp.filter_state.input_active = false,
        FilterAction::Backspace => {
            lp.filter_state.input.pop();
            lp.filter_state.apply_input(&lp.log);
        }
        FilterAction::Insert(ch) => {
            lp.filter_state.input.push(ch);
            lp.filter_state.apply_input(&lp.log);
        }
    }
}

fn apply_cursor_action(lp: &mut LogPreview, action: CursorAction) {
    match action {
        CursorAction::ToNewest => {
            lp.filter_state.cursor = 0;
            lp.filter_state.scroll = 0;
        }
        CursorAction::ToOldest => lp.filter_state.scroll_to_bottom(),
        CursorAction::Down(count) => lp.filter_state.scroll_down(count),
        CursorAction::Up(count) => lp.filter_state.scroll_up(count),
    }
}

fn apply_column_action(lp: &mut LogPreview, action: ColumnAction) {
    match action {
        ColumnAction::MoveLeft => lp.filter_state.move_col_left(),
        ColumnAction::MoveRight => lp.filter_state.move_col_right(),
        ColumnAction::HideSelected => lp.filter_state.hide_selected_col(),
        ColumnAction::IsolateSelected => lp.filter_state.isolate_selected_col(),
        ColumnAction::OpenPicker => {
            let fields = lp.log.all_fields.clone();
            lp.filter_state.open_col_modal(&fields);
        }
        ColumnAction::Resize(delta) => lp.filter_state.resize_selected_col(delta),
    }
}

fn apply_modal_action(lp: &mut LogPreview, action: ModalAction) {
    let Some(modal) = &mut lp.filter_state.col_modal else {
        return;
    };
    let field_count = modal.checked.len();

    match action {
        ModalAction::Close => {
            lp.filter_state.col_modal = None;
        }
        ModalAction::Apply => {
            let all_fields = lp.log.all_fields.clone();
            lp.filter_state.apply_modal_changes(&all_fields);
        }
        ModalAction::Down => {
            if field_count > 0 {
                let modal = lp.filter_state.col_modal.as_mut().expect("modal exists");
                modal.cursor = (modal.cursor + 1).min(field_count - 1);
            }
        }
        ModalAction::Up => {
            if let Some(modal) = &mut lp.filter_state.col_modal {
                modal.cursor = modal.cursor.saturating_sub(1);
            }
        }
        ModalAction::Toggle { advance } => {
            if let Some(modal) = &mut lp.filter_state.col_modal {
                if let Some(checked) = modal.checked.get_mut(modal.cursor) {
                    *checked = !*checked;
                }
                if advance && modal.cursor + 1 < field_count {
                    modal.cursor += 1;
                }
            }
        }
    }
}

fn copy_entries(lp: &mut LogPreview, raw: bool) {
    let filter_state = &lp.filter_state;
    let entries = &lp.log.entries;

    let text: String = if !filter_state.marked.is_empty() {
        let lines: Vec<String> = filter_state
            .cached_matches
            .iter()
            .filter(|&&index| filter_state.marked.contains(&index))
            .map(|&index| {
                if raw {
                    entries[index].raw.clone()
                } else {
                    format_entry_visible(&entries[index], &filter_state.visible_cols)
                }
            })
            .collect();
        lines.join("\n")
    } else {
        let cursor = filter_state
            .cursor
            .min(filter_state.cached_matches.len().saturating_sub(1));
        match filter_state.cached_matches.get(cursor) {
            Some(&index) => {
                if raw {
                    entries[index].raw.clone()
                } else {
                    format_entry_visible(&entries[index], &filter_state.visible_cols)
                }
            }
            None => return,
        }
    };

    if text.is_empty() {
        return;
    }
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text);
    }
    lp.filter_state.clear_marks();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::structured_log::actions::LogViewerAction;
    use crate::preview::structured_log::{LogEntry, LogFormat, StructuredLog};
    use std::time::{Duration, Instant};

    fn entry(fields: &[(&str, &str)], raw: &str) -> LogEntry {
        LogEntry {
            fields: fields
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            raw: raw.to_string(),
        }
    }

    #[test]
    fn preview_content_initializes_reverse_match_order() {
        let log = StructuredLog {
            entries: vec![
                entry(&[("level", "info")], "level=info"),
                entry(&[("level", "warn")], "level=warn"),
            ],
            total_lines: 2,
            all_fields: vec!["level".to_string()],
            format: LogFormat::Logfmt,
        };

        let PreviewContent::StructuredLog(preview) = preview_content(log) else {
            panic!("expected structured log preview");
        };

        assert_eq!(preview.filter_state.cached_matches, vec![1, 0]);
        assert_eq!(preview.filter_state.visible_cols.len(), 1);
        assert_eq!(preview.filter_state.visible_cols[0].field, "level");
    }

    #[test]
    fn append_entries_tracks_total_lines_even_after_capacity() {
        let log = StructuredLog {
            entries: vec![entry(&[("level", "info")], "level=info")],
            total_lines: 1,
            all_fields: vec!["level".to_string()],
            format: LogFormat::Logfmt,
        };
        let PreviewContent::StructuredLog(mut preview) = preview_content(log) else {
            panic!("expected structured log preview");
        };

        append_entries(
            &mut preview,
            vec![
                entry(&[("msg", "first")], "msg=first"),
                entry(&[("msg", "second")], "msg=second"),
            ],
            2,
        );

        assert_eq!(preview.log.total_lines, 3);
        assert_eq!(preview.log.entries.len(), 2);
        assert_eq!(preview.filter_state.cached_matches, vec![1, 0]);
        assert!(preview.log.all_fields.iter().any(|field| field == "msg"));
    }

    #[test]
    fn filter_actions_recompute_matches() {
        let PreviewContent::StructuredLog(mut preview) = preview_content(StructuredLog {
            entries: vec![
                entry(&[("level", "info")], "level=info"),
                entry(&[("level", "warn")], "level=warn"),
            ],
            total_lines: 2,
            all_fields: vec!["level".to_string()],
            format: LogFormat::Logfmt,
        }) else {
            panic!("expected structured log preview");
        };

        apply_action(
            &mut preview,
            LogViewerAction::Filter(FilterAction::StartEditing),
            false,
        );
        apply_action(
            &mut preview,
            LogViewerAction::Filter(FilterAction::Insert('w')),
            false,
        );

        assert_eq!(preview.filter_state.cached_matches, vec![1]);
    }

    #[test]
    fn modal_actions_toggle_and_apply() {
        let PreviewContent::StructuredLog(mut preview) = preview_content(StructuredLog {
            entries: vec![entry(
                &[("level", "info"), ("msg", "hello")],
                "level=info msg=hello",
            )],
            total_lines: 1,
            all_fields: vec!["level".to_string(), "msg".to_string()],
            format: LogFormat::Logfmt,
        }) else {
            panic!("expected structured log preview");
        };
        preview
            .filter_state
            .open_col_modal(&preview.log.all_fields.clone());

        let outcome = apply_action(
            &mut preview,
            LogViewerAction::Modal(ModalAction::Toggle { advance: true }),
            false,
        );
        assert!(matches!(outcome, LogViewerOutcome::None));
        assert_eq!(
            preview
                .filter_state
                .col_modal
                .as_ref()
                .map(|modal| modal.cursor),
            Some(1)
        );

        apply_action(
            &mut preview,
            LogViewerAction::Modal(ModalAction::Apply),
            false,
        );
        assert!(preview.filter_state.col_modal.is_none());
    }

    #[test]
    #[ignore = "performance smoke test"]
    fn large_log_filtering_smoke_test() {
        let entries = (0..20_000)
            .map(|i| {
                entry(
                    &[
                        ("level", if i % 2 == 0 { "info" } else { "warn" }),
                        ("msg", &format!("message-{i}")),
                    ],
                    "raw",
                )
            })
            .collect();

        let PreviewContent::StructuredLog(mut preview) = preview_content(StructuredLog {
            entries,
            total_lines: 20_000,
            all_fields: vec!["level".to_string(), "msg".to_string()],
            format: LogFormat::Logfmt,
        }) else {
            panic!("expected structured log preview");
        };

        let started = Instant::now();
        apply_action(
            &mut preview,
            LogViewerAction::Filter(FilterAction::StartEditing),
            false,
        );
        for ch in "warn".chars() {
            apply_action(
                &mut preview,
                LogViewerAction::Filter(FilterAction::Insert(ch)),
                false,
            );
        }

        assert_eq!(preview.filter_state.cached_matches.len(), 10_000);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    #[ignore = "performance smoke test"]
    fn frequent_append_smoke_test() {
        let PreviewContent::StructuredLog(mut preview) = preview_content(StructuredLog {
            entries: vec![],
            total_lines: 0,
            all_fields: vec!["level".to_string(), "msg".to_string()],
            format: LogFormat::Logfmt,
        }) else {
            panic!("expected structured log preview");
        };

        let started = Instant::now();
        for batch in 0..200 {
            let entries = (0..100)
                .map(|i| {
                    let idx = batch * 100 + i;
                    entry(
                        &[("level", "info"), ("msg", &format!("message-{idx}"))],
                        "raw",
                    )
                })
                .collect();
            append_entries(&mut preview, entries, 50_000);
        }

        assert_eq!(preview.log.total_lines, 20_000);
        assert_eq!(preview.log.entries.len(), 20_000);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
