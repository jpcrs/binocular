use crate::preview::types::LogPreview;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogViewerOutcome {
    None,
    ExitApp,
    FocusSearch,
}

pub(crate) enum LogViewerAction {
    Filter(FilterAction),
    Cursor(CursorAction),
    Column(ColumnAction),
    Modal(ModalAction),
    TogglePause,
    ToggleMark,
    Copy { raw: bool },
    ResetView,
    Exit,
}

pub(crate) enum FilterAction {
    StartEditing,
    StopEditing,
    Backspace,
    Insert(char),
}

pub(crate) enum CursorAction {
    ToNewest,
    ToOldest,
    Down(usize),
    Up(usize),
}

pub(crate) enum ColumnAction {
    MoveLeft,
    MoveRight,
    HideSelected,
    IsolateSelected,
    OpenPicker,
    Resize(i32),
}

pub(crate) enum ModalAction {
    Close,
    Apply,
    Down,
    Up,
    Toggle { advance: bool },
}

pub(crate) fn action_for_key(lp: &LogPreview, key: KeyEvent) -> Option<LogViewerAction> {
    if lp.filter_state.col_modal.is_some() {
        return modal_action_for_key(key).map(LogViewerAction::Modal);
    }

    if lp.filter_state.input_active {
        return filter_action_for_key(key).map(LogViewerAction::Filter);
    }

    match key.code {
        KeyCode::Char('/') => Some(LogViewerAction::Filter(FilterAction::StartEditing)),
        KeyCode::Char('r') => Some(LogViewerAction::ResetView),
        KeyCode::Char('G') => Some(LogViewerAction::Cursor(CursorAction::ToOldest)),
        KeyCode::Char('g') => Some(LogViewerAction::Cursor(CursorAction::ToNewest)),
        KeyCode::Char('j') | KeyCode::Down => Some(LogViewerAction::Cursor(CursorAction::Down(1))),
        KeyCode::Char('k') | KeyCode::Up => Some(LogViewerAction::Cursor(CursorAction::Up(1))),
        KeyCode::Char('d') => Some(LogViewerAction::Cursor(CursorAction::Down(20))),
        KeyCode::Char('u') => Some(LogViewerAction::Cursor(CursorAction::Up(20))),
        KeyCode::Char('h') | KeyCode::Left => Some(LogViewerAction::Column(ColumnAction::MoveLeft)),
        KeyCode::Char('l') | KeyCode::Right => {
            Some(LogViewerAction::Column(ColumnAction::MoveRight))
        }
        KeyCode::Char('H') => Some(LogViewerAction::Column(ColumnAction::HideSelected)),
        KeyCode::Char('o') => Some(LogViewerAction::Column(ColumnAction::IsolateSelected)),
        KeyCode::Char('a') => Some(LogViewerAction::Column(ColumnAction::OpenPicker)),
        KeyCode::Char('<') => Some(LogViewerAction::Column(ColumnAction::Resize(-5))),
        KeyCode::Char('>') => Some(LogViewerAction::Column(ColumnAction::Resize(5))),
        KeyCode::Char('p') => Some(LogViewerAction::TogglePause),
        KeyCode::Tab => Some(LogViewerAction::ToggleMark),
        KeyCode::Char('y') => Some(LogViewerAction::Copy { raw: false }),
        KeyCode::Char('Y') => Some(LogViewerAction::Copy { raw: true }),
        KeyCode::Esc | KeyCode::Char('q') => Some(LogViewerAction::Exit),
        _ => None,
    }
}

fn filter_action_for_key(key: KeyEvent) -> Option<FilterAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => Some(FilterAction::StopEditing),
        KeyCode::Backspace => Some(FilterAction::Backspace),
        KeyCode::Char(c) => Some(FilterAction::Insert(c)),
        _ => None,
    }
}

fn modal_action_for_key(key: KeyEvent) -> Option<ModalAction> {
    match key.code {
        KeyCode::Esc => Some(ModalAction::Close),
        KeyCode::Enter => Some(ModalAction::Apply),
        KeyCode::Char('j') | KeyCode::Down => Some(ModalAction::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(ModalAction::Up),
        KeyCode::Char(' ') => Some(ModalAction::Toggle { advance: false }),
        KeyCode::Tab => Some(ModalAction::Toggle { advance: true }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::structured_log::{preview_content, LogEntry, LogFormat, StructuredLog};
    use crate::preview::PreviewContent;
    use crossterm::event::KeyModifiers;

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
    fn normal_mode_keys_map_to_explicit_actions() {
        let PreviewContent::StructuredLog(preview) = preview_content(StructuredLog {
            entries: vec![entry(&[("level", "info")], "level=info")],
            total_lines: 1,
            all_fields: vec!["level".to_string()],
            format: LogFormat::Logfmt,
        }) else {
            panic!("expected structured log preview");
        };

        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(matches!(
            action_for_key(&preview, key),
            Some(LogViewerAction::Cursor(CursorAction::Down(20)))
        ));

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(matches!(
            action_for_key(&preview, key),
            Some(LogViewerAction::Column(ColumnAction::OpenPicker))
        ));
    }
}
