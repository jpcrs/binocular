use crate::app::{App, InputMode};
use crossterm::event::{KeyCode, KeyEvent};

use super::command_parser::{push_count_digit, take_count, OperatorMotion};
use super::common_actions::parse_common_normal_action;
use super::line_editor;
use super::normal_action_handler::{self, CommonActionTarget};
use super::operator_handler::{self, PendingOperatorResult, PendingOperatorTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchInputResult {
    QueryChanged,
    ListUp(usize),
    ListDown(usize),
    None,
    Quit,
    Select,
}

pub fn handle_search_input(key: KeyEvent, app: &mut App) -> SearchInputResult {
    let query = &mut app.search_session.query.text;
    let cursor = &mut app.search_session.query.cursor;
    let mode = &mut app.search_session.query.mode;
    let count_buffer = &mut app.search_session.query.count_buffer;
    let pending_op = &mut app.search_session.query.pending_op;
    let pending_modifier = &mut app.search_session.query.pending_modifier;

    line_editor::clamp_cursor_insert(cursor, query);

    if *mode == InputMode::Insert {
        return handle_insert_mode(key, query, cursor, mode);
    }

    if push_count_digit(count_buffer, key) {
        return SearchInputResult::None;
    }

    let count = take_count(count_buffer);

    if pending_op.is_some() && pending_modifier.is_some() {
        return handle_text_object(key, query, cursor, mode, pending_op, pending_modifier);
    }

    if let Some(op) = *pending_op {
        return handle_operator_pending(
            key,
            query,
            cursor,
            mode,
            pending_op,
            pending_modifier,
            op,
            count,
        );
    }

    handle_normal_mode(key, query, cursor, mode, pending_op, count)
}

fn handle_insert_mode(
    key: KeyEvent,
    query: &mut String,
    cursor: &mut usize,
    mode: &mut InputMode,
) -> SearchInputResult {
    match key.code {
        KeyCode::Esc => {
            *mode = InputMode::Normal;
            if line_editor::char_count(query) > 0 && *cursor > 0 {
                *cursor -= 1;
            }
            SearchInputResult::None
        }
        KeyCode::Up => SearchInputResult::ListUp(1),
        KeyCode::Down => SearchInputResult::ListDown(1),
        KeyCode::Left => {
            if *cursor > 0 {
                *cursor -= 1;
            }
            SearchInputResult::None
        }
        KeyCode::Right => {
            line_editor::move_right_insert(cursor, query);
            SearchInputResult::None
        }
        KeyCode::Char(c) => {
            line_editor::insert_char(query, cursor, c);
            SearchInputResult::QueryChanged
        }
        KeyCode::Backspace => {
            if line_editor::backspace(query, cursor) {
                SearchInputResult::QueryChanged
            } else {
                SearchInputResult::None
            }
        }
        KeyCode::Delete => {
            if line_editor::delete_char_at_cursor(query, *cursor) {
                SearchInputResult::QueryChanged
            } else {
                SearchInputResult::None
            }
        }
        KeyCode::Enter => SearchInputResult::Select,
        _ => SearchInputResult::None,
    }
}

fn handle_text_object(
    key: KeyEvent,
    query: &mut String,
    cursor: &mut usize,
    mode: &mut InputMode,
    pending_op: &mut Option<char>,
    pending_modifier: &mut Option<char>,
) -> SearchInputResult {
    let op = pending_op.unwrap_or(' ');
    let modifier = pending_modifier.unwrap_or('i');
    *pending_op = None;
    *pending_modifier = None;

    let changed = match key.code {
        KeyCode::Char('w') => {
            if let Some((start, end)) =
                line_editor::find_word_bounds(query, *cursor, modifier == 'a')
            {
                let changed = line_editor::replace_char_range(query, start, end);
                *cursor = start;
                changed
            } else {
                false
            }
        }
        KeyCode::Char('W') => {
            if let Some((start, end)) =
                line_editor::find_big_word_bounds(query, *cursor, modifier == 'a')
            {
                let changed = line_editor::replace_char_range(query, start, end);
                *cursor = start;
                changed
            } else {
                false
            }
        }
        _ => false,
    };

    if !changed {
        return SearchInputResult::None;
    }

    if op == 'c' {
        *mode = InputMode::Insert;
    }

    normalize_cursor(query, cursor, *mode);
    SearchInputResult::QueryChanged
}

fn handle_operator_pending(
    key: KeyEvent,
    query: &mut String,
    cursor: &mut usize,
    mode: &mut InputMode,
    pending_op: &mut Option<char>,
    pending_modifier: &mut Option<char>,
    op: char,
    count: usize,
) -> SearchInputResult {
    let result = {
        let mut target = SearchBarOperatorTarget {
            query,
            cursor,
            mode,
            pending_op,
            pending_modifier,
        };
        operator_handler::handle_pending_operator(&mut target, key, op, count)
    };

    normalize_cursor(query, cursor, *mode);
    match result {
        PendingOperatorResult::AwaitingMore => SearchInputResult::None,
        PendingOperatorResult::Applied { changed } => {
            if changed {
                SearchInputResult::QueryChanged
            } else {
                SearchInputResult::None
            }
        }
        PendingOperatorResult::Cleared | PendingOperatorResult::Unhandled => {
            *pending_op = None;
            *pending_modifier = None;
            SearchInputResult::None
        }
    }
}

fn handle_normal_mode(
    key: KeyEvent,
    query: &mut String,
    cursor: &mut usize,
    mode: &mut InputMode,
    pending_op: &mut Option<char>,
    count: usize,
) -> SearchInputResult {
    match key.code {
        KeyCode::Esc => return SearchInputResult::Quit,
        KeyCode::Enter => return SearchInputResult::Select,
        KeyCode::Char('j') | KeyCode::Down => return SearchInputResult::ListDown(count),
        KeyCode::Char('k') | KeyCode::Up => return SearchInputResult::ListUp(count),
        _ => {}
    }

    let Some(action) = parse_common_normal_action(key) else {
        return SearchInputResult::None;
    };
    apply_common_action(action, query, cursor, mode, pending_op, count)
}

fn apply_common_action(
    action: super::common_actions::CommonNormalAction,
    query: &mut String,
    cursor: &mut usize,
    mode: &mut InputMode,
    pending_op: &mut Option<char>,
    count: usize,
) -> SearchInputResult {
    let changed = {
        let mut target = SearchBarTarget {
            query,
            cursor,
            mode,
            pending_op,
        };
        normal_action_handler::apply_common_normal_action(&mut target, action, count)
    };

    normalize_cursor(query, cursor, *mode);
    if changed {
        SearchInputResult::QueryChanged
    } else {
        SearchInputResult::None
    }
}

struct SearchBarTarget<'a> {
    query: &'a mut String,
    cursor: &'a mut usize,
    mode: &'a mut InputMode,
    pending_op: &'a mut Option<char>,
}

struct SearchBarOperatorTarget<'a> {
    query: &'a mut String,
    cursor: &'a mut usize,
    mode: &'a mut InputMode,
    pending_op: &'a mut Option<char>,
    pending_modifier: &'a mut Option<char>,
}

impl CommonActionTarget for SearchBarTarget<'_> {
    fn move_left(&mut self, count: usize) {
        *self.cursor = self.cursor.saturating_sub(count);
    }

    fn move_right(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_right_normal(self.cursor, self.query);
        }
    }

    fn move_start_of_line(&mut self) {
        line_editor::move_start_of_line(self.cursor);
    }

    fn move_end_of_line(&mut self) {
        line_editor::move_end_of_line_normal(self.cursor, self.query);
    }

    fn move_first_non_blank(&mut self) {
        line_editor::move_first_non_blank(self.cursor, self.query);
    }

    fn move_word_forward(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_word_forward(self.cursor, self.query);
        }
    }

    fn move_word_end_forward(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_word_end_forward(self.cursor, self.query);
        }
    }

    fn move_word_backward(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_word_backward(self.cursor, self.query);
        }
    }

    fn move_big_word_forward(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_big_word_forward(self.cursor, self.query);
        }
    }

    fn move_big_word_backward(&mut self, count: usize) {
        for _ in 0..count {
            line_editor::move_big_word_backward(self.cursor, self.query);
        }
    }

    fn enter_insert_before(&mut self) {
        *self.mode = InputMode::Insert;
        let len = line_editor::char_count(self.query);
        if *self.cursor == len.saturating_sub(1) && len > 0 {
            *self.cursor = len;
        }
    }

    fn enter_insert_after(&mut self) {
        *self.mode = InputMode::Insert;
        line_editor::move_right_insert(self.cursor, self.query);
    }

    fn enter_insert_at_end(&mut self) {
        *self.mode = InputMode::Insert;
        line_editor::move_end_of_line_insert(self.cursor, self.query);
    }

    fn enter_insert_at_first_non_blank(&mut self) {
        *self.mode = InputMode::Insert;
        line_editor::move_first_non_blank(self.cursor, self.query);
    }

    fn delete_char_under_cursor(&mut self, count: usize) -> bool {
        let mut changed = false;
        for _ in 0..count {
            changed |= line_editor::delete_char_at_cursor(self.query, *self.cursor);
        }
        changed
    }

    fn delete_to_end_of_line(&mut self) -> bool {
        line_editor::truncate_from_cursor(self.query, *self.cursor)
    }

    fn change_to_end_of_line(&mut self) -> bool {
        if *self.cursor < line_editor::char_count(self.query) {
            let _ = line_editor::truncate_from_cursor(self.query, *self.cursor);
        }
        *self.mode = InputMode::Insert;
        true
    }

    fn substitute_char(&mut self) -> bool {
        if *self.cursor < line_editor::char_count(self.query) {
            let _ = line_editor::delete_char_at_cursor(self.query, *self.cursor);
        }
        *self.mode = InputMode::Insert;
        true
    }

    fn substitute_line(&mut self) -> bool {
        self.query.clear();
        *self.cursor = 0;
        *self.mode = InputMode::Insert;
        true
    }

    fn start_operator(&mut self, op: char) {
        *self.pending_op = Some(op);
    }
}

impl PendingOperatorTarget for SearchBarOperatorTarget<'_> {
    fn set_modifier(&mut self, modifier: char) -> bool {
        if matches!(modifier, 'i' | 'a') {
            *self.pending_modifier = Some(modifier);
            true
        } else {
            false
        }
    }

    fn repeat_operator(&mut self, op: char, _count: usize) -> bool {
        match op {
            'c' => {
                self.query.clear();
                *self.cursor = 0;
                *self.mode = InputMode::Insert;
                true
            }
            'd' => {
                self.query.clear();
                *self.cursor = 0;
                true
            }
            _ => false,
        }
    }

    fn apply_motion(&mut self, op: char, motion: OperatorMotion, count: usize) -> bool {
        let changed = match motion {
            OperatorMotion::StartOfLine => {
                if *self.cursor > 0 {
                    let changed = line_editor::replace_char_range(self.query, 0, *self.cursor);
                    *self.cursor = 0;
                    changed
                } else {
                    false
                }
            }
            OperatorMotion::EndOfLine => {
                line_editor::truncate_from_cursor(self.query, *self.cursor)
            }
            OperatorMotion::WordForward | OperatorMotion::WordEndForward => {
                let len = line_editor::char_count(self.query);
                let mut end_exclusive = *self.cursor;
                for _ in 0..count {
                    let from = end_exclusive.min(len.saturating_sub(1));
                    let end = line_editor::find_word_end(self.query, from);
                    end_exclusive = (end + 1).min(len);
                }
                if end_exclusive > *self.cursor {
                    line_editor::replace_char_range(self.query, *self.cursor, end_exclusive)
                } else {
                    false
                }
            }
            OperatorMotion::WordBackward => {
                let mut start = *self.cursor;
                for _ in 0..count {
                    start = line_editor::find_prev_word_start(self.query, start);
                }
                let changed = if start < *self.cursor {
                    line_editor::replace_char_range(self.query, start, *self.cursor)
                } else {
                    false
                };
                *self.cursor = start;
                changed
            }
        };

        if op == 'c' {
            *self.mode = InputMode::Insert;
        }
        changed
    }

    fn clear_pending(&mut self) {
        *self.pending_op = None;
        *self.pending_modifier = None;
    }
}

fn normalize_cursor(query: &str, cursor: &mut usize, mode: InputMode) {
    if mode == InputMode::Insert {
        line_editor::clamp_cursor_insert(cursor, query);
    } else {
        line_editor::clamp_cursor_normal(cursor, query);
    }
}
