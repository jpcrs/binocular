use crate::app::{App, InputMode};
use crate::preview::PreviewContent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::command_parser::{push_count_digit, take_count};
use super::commands::{execute_command, save_file};
use super::common_actions::parse_common_normal_action;
use super::edit::{
    delete_char, delete_char_under_cursor, delete_line, delete_line_content, delete_range,
    delete_to_end_of_line, insert_char, insert_newline, open_line_above, open_line_below,
};
use super::motion::{
    move_big_word_backward, move_big_word_end_forward, move_big_word_forward, move_down,
    move_end_of_line, move_end_of_line_for_insert, move_first_non_blank, move_half_page_down,
    move_half_page_up, move_left, move_matching_bracket, move_right, move_start_of_line,
    move_to_end_of_file, move_to_start_of_file, move_up, move_word_backward, move_word_end_forward,
    move_word_forward, perform_char_search,
};
use super::normal_action_handler::{self, CommonActionTarget};
use super::operator_handler::{self, PendingOperatorResult, PendingOperatorTarget};
use super::search::perform_search;
use super::text_objects::{
    find_inner_pair_range, find_inner_quote_range, find_inner_word_range, select_inner_word,
};
use super::undo::{perform_redo, perform_undo};
use super::utils::{adjust_scroll, ensure_cursor_in_bounds, line_len};
use super::yank::{yank_line, yank_range_content, yank_selection};

pub fn handle_input(key: KeyEvent, app: &mut App) {
    if app.preview_session.preview.state.search_active {
        handle_search_mode(key, app);
        return;
    }

    if app.preview_session.preview.state.mode == InputMode::Command {
        handle_command_mode(key, app);
        return;
    }

    if app.preview_session.preview.state.mode == InputMode::Insert {
        handle_insert_mode(key, app);
        return;
    }

    if let Some((forward, stored_count)) = app.preview_session.preview.state.waiting_for_char_search
    {
        if let KeyCode::Char(c) = key.code {
            app.preview_session.preview.state.last_char_search = Some((c, forward));
            perform_char_search(app, c, forward, stored_count);
        }
        app.preview_session.preview.state.waiting_for_char_search = None;
        return;
    }

    if push_count_digit(&mut app.preview_session.preview.state.input_buffer, key) {
        return;
    }

    let count = take_count(&mut app.preview_session.preview.state.input_buffer);

    if let Some(op) = app.preview_session.preview.state.pending_operator {
        handle_operator_pending(key, app, op, count);
    } else {
        handle_normal_visual_mode(key, app, count);
    }

    ensure_cursor_in_bounds(app);
    adjust_scroll(app);
    refresh_text_preview_if_needed(app);
}

fn handle_search_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.preview_session.preview.state.search_active = false;
            app.preview_session.preview.state.search_query.clear();
            if app.preview_session.preview.state.mode == InputMode::Visual
                || app.preview_session.preview.state.mode == InputMode::VisualLine
            {
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            }
        }
        KeyCode::Enter => {
            app.preview_session.preview.state.search_active = false;
            perform_search(app, true);
            ensure_cursor_in_bounds(app);
            adjust_scroll(app);
        }
        KeyCode::Char(c) => {
            app.preview_session.preview.state.search_query.push(c);
        }
        KeyCode::Backspace => {
            app.preview_session.preview.state.search_query.pop();
        }
        _ => {}
    }
}

fn handle_command_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.preview_session.preview.state.mode = InputMode::Normal;
            app.preview_session.preview.state.command_buffer.clear();
        }
        KeyCode::Enter => {
            let cmd = app.preview_session.preview.state.command_buffer.clone();
            app.preview_session.preview.state.command_buffer.clear();
            app.preview_session.preview.state.mode = InputMode::Normal;
            execute_command(app, &cmd);
        }
        KeyCode::Char(c) => {
            app.preview_session.preview.state.command_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.preview_session.preview.state.command_buffer.pop();
            if app.preview_session.preview.state.command_buffer.is_empty() {
                app.preview_session.preview.state.mode = InputMode::Normal;
            }
        }
        _ => {}
    }
}

fn handle_insert_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            let path = std::path::PathBuf::from(app.preview_file_path().unwrap_or(""));
            if let Some(PreviewContent::RichText(text_file)) =
                &mut app.preview_session.preview.content
            {
                if text_file.dirty {
                    crate::preview::regenerate_lines(text_file, &path);
                }
            }
            app.preview_session.preview.state.mode = InputMode::Normal;
            if app.preview_session.preview.state.cursor_char > 0 {
                app.preview_session.preview.state.cursor_char -= 1;
            }
        }
        KeyCode::Char(c) => {
            insert_char(app, c);
        }
        KeyCode::Enter => {
            insert_newline(app);
        }
        KeyCode::Backspace => {
            delete_char(app);
        }
        KeyCode::Delete => {
            delete_char_under_cursor(app);
        }
        KeyCode::Left => {
            if app.preview_session.preview.state.cursor_char > 0 {
                app.preview_session.preview.state.cursor_char -= 1;
            }
        }
        KeyCode::Right => {
            let len = line_len(app, app.preview_session.preview.state.cursor_line);
            if app.preview_session.preview.state.cursor_char < len {
                app.preview_session.preview.state.cursor_char += 1;
            }
        }
        KeyCode::Up => {
            if app.preview_session.preview.state.cursor_line > 0 {
                app.preview_session.preview.state.cursor_line -= 1;
                clamp_cursor_to_line(app);
            }
        }
        KeyCode::Down => {
            let line_count = super::utils::get_line_count(app);
            if app.preview_session.preview.state.cursor_line + 1 < line_count {
                app.preview_session.preview.state.cursor_line += 1;
                clamp_cursor_to_line(app);
            }
        }
        _ => {}
    }
    ensure_cursor_in_bounds(app);
    adjust_scroll(app);
}

fn refresh_text_preview_if_needed(app: &mut App) {
    if app.preview_session.preview.state.mode == InputMode::Insert {
        return;
    }

    let path = std::path::PathBuf::from(app.preview_file_path().unwrap_or(""));
    if let Some(PreviewContent::RichText(text_file)) = &mut app.preview_session.preview.content {
        if text_file.dirty {
            crate::preview::regenerate_lines(text_file, &path);
        }
    }
}

fn clamp_cursor_to_line(app: &mut App) {
    let len = line_len(app, app.preview_session.preview.state.cursor_line);
    if app.preview_session.preview.state.cursor_char > len {
        app.preview_session.preview.state.cursor_char = len;
    }
}

fn execute_operator_on_range(
    app: &mut App,
    op: char,
    start: (usize, usize),
    end: (usize, usize),
    msg: &str,
) {
    match op {
        'd' => {
            delete_range(app, start, end);
            app.preview_session.preview.state.status_message =
                Some((format!("Deleted {}", msg), std::time::Instant::now()));
        }
        'c' => {
            delete_range(app, start, end);
            app.preview_session.preview.state.mode = InputMode::Insert;
            app.preview_session.preview.state.status_message =
                Some((format!("Changed {}", msg), std::time::Instant::now()));
        }
        'y' => {
            yank_range_content(app, start, end);
            app.preview_session.preview.state.status_message =
                Some((format!("Yanked {}", msg), std::time::Instant::now()));
        }
        _ => return,
    }
    app.preview_session.preview.state.pending_operator = None;
    app.preview_session.preview.state.pending_object_modifier = None;
}

fn handle_operator_pending(key: KeyEvent, app: &mut App, op: char, count: usize) {
    if app
        .preview_session
        .preview
        .state
        .pending_object_modifier
        .is_none()
    {
        let shared_result = {
            let mut target = PreviewOperatorTarget { app };
            operator_handler::handle_pending_operator(&mut target, key, op, count)
        };

        match shared_result {
            PendingOperatorResult::AwaitingMore
            | PendingOperatorResult::Applied { .. }
            | PendingOperatorResult::Cleared => return,
            PendingOperatorResult::Unhandled => {}
        }
    }

    match key.code {
        KeyCode::Char('\'' | '"' | '`') => {
            if let KeyCode::Char(quote) = key.code {
                if let Some((start, end)) = find_inner_quote_range(app, quote) {
                    execute_operator_on_range(app, op, start, end, "inner object");
                }
            }
        }
        KeyCode::Char(c) => {
            if app.preview_session.preview.state.pending_object_modifier == Some('i') {
                let range = match c {
                    'w' => find_inner_word_range(app),
                    '(' | ')' | 'b' => find_inner_pair_range(app, '(', ')'),
                    '[' | ']' => find_inner_pair_range(app, '[', ']'),
                    '{' | '}' | 'B' => find_inner_pair_range(app, '{', '}'),
                    '<' | '>' => find_inner_pair_range(app, '<', '>'),
                    _ => None,
                };
                if let Some((start, end)) = range {
                    execute_operator_on_range(app, op, start, end, "inner object");
                }
                app.preview_session.preview.state.pending_operator = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            } else {
                app.preview_session.preview.state.pending_operator = None;
            }
        }
        _ => {
            app.preview_session.preview.state.pending_operator = None;
            app.preview_session.preview.state.pending_object_modifier = None;
        }
    }
}

fn handle_normal_visual_mode(key: KeyEvent, app: &mut App, count: usize) {
    if app.preview_session.preview.state.mode == InputMode::Normal {
        if let Some(action) = parse_common_normal_action(key) {
            let mut target = PreviewNormalTarget { app };
            normal_action_handler::apply_common_normal_action(&mut target, action, count);
            return;
        }
    }

    match key.code {
        KeyCode::Char('y') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.preview_session.preview.state.mode == InputMode::Visual
                || app.preview_session.preview.state.mode == InputMode::VisualLine
            {
                yank_selection(app);
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            } else {
                app.preview_session.preview.state.pending_operator = Some('y');
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_file(app);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_half_page_down(app, count)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_half_page_up(app, count)
        }
        KeyCode::Char('u') => {
            for _ in 0..count {
                perform_undo(app);
            }
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..count {
                perform_redo(app);
            }
        }
        KeyCode::Char('U') => {
            for _ in 0..count {
                perform_redo(app);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => move_down(app, count),
        KeyCode::Char('k') | KeyCode::Up => move_up(app, count),
        KeyCode::Char('H') => move_start_of_line(app),
        KeyCode::Char('L') => move_end_of_line(app),
        KeyCode::Char('G') => move_to_end_of_file(app, count),
        KeyCode::Char('g') => {
            app.preview_session.preview.state.pending_operator = Some('g');
        }
        KeyCode::Char('/') => {
            app.preview_session.preview.state.search_active = true;
            app.preview_session.preview.state.search_query.clear();
        }
        KeyCode::Char(':') => {
            app.preview_session.preview.state.mode = InputMode::Command;
            app.preview_session.preview.state.command_buffer.clear();
        }
        KeyCode::Char('n') => {
            for _ in 0..count {
                perform_search(app, true);
            }
        }
        KeyCode::Char('N') => {
            for _ in 0..count {
                perform_search(app, false);
            }
        }
        KeyCode::Char('E') => move_big_word_end_forward(app, count),
        KeyCode::Char('^') | KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::SHIFT) => {
            move_first_non_blank(app)
        }
        KeyCode::Char('f') => {
            app.preview_session.preview.state.waiting_for_char_search = Some((true, count));
        }
        KeyCode::Char('F') => {
            app.preview_session.preview.state.waiting_for_char_search = Some((false, count));
        }
        KeyCode::Char(';') => {
            if let Some((c, forward)) = app.preview_session.preview.state.last_char_search {
                perform_char_search(app, c, forward, count);
            }
        }
        KeyCode::Char('%') => move_matching_bracket(app),
        KeyCode::Char('v') => {
            if app.preview_session.preview.state.mode == InputMode::Normal {
                app.preview_session.preview.state.mode = InputMode::Visual;
                app.preview_session.preview.state.selection_start = Some((
                    app.preview_session.preview.state.cursor_line,
                    app.preview_session.preview.state.cursor_char,
                ));
            } else {
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            }
        }
        KeyCode::Char('V') => {
            if app.preview_session.preview.state.mode == InputMode::Normal {
                app.preview_session.preview.state.mode = InputMode::VisualLine;
                app.preview_session.preview.state.selection_start =
                    Some((app.preview_session.preview.state.cursor_line, 0));
            } else {
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            }
        }
        KeyCode::Esc => {
            if app.preview_session.preview.state.mode == InputMode::Visual
                || app.preview_session.preview.state.mode == InputMode::VisualLine
            {
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            }
        }
        KeyCode::Char('Y') => {
            if app.preview_session.preview.state.mode == InputMode::Visual
                || app.preview_session.preview.state.mode == InputMode::VisualLine
            {
                yank_selection(app);
                app.preview_session.preview.state.mode = InputMode::Normal;
                app.preview_session.preview.state.selection_start = None;
                app.preview_session.preview.state.pending_object_modifier = None;
            } else {
                yank_line(app);
            }
        }
        KeyCode::Char('i') => {
            if app.preview_session.preview.state.mode == InputMode::Visual {
                app.preview_session.preview.state.pending_object_modifier = Some('i');
            }
        }
        KeyCode::Char('o') => {
            if app.preview_session.preview.state.mode == InputMode::Normal {
                open_line_below(app);
                app.preview_session.preview.state.mode = InputMode::Insert;
            }
        }
        KeyCode::Char('O') => {
            if app.preview_session.preview.state.mode == InputMode::Normal {
                open_line_above(app);
                app.preview_session.preview.state.mode = InputMode::Insert;
            }
        }
        KeyCode::Char('w') => {
            if app.preview_session.preview.state.mode == InputMode::Visual
                && app.preview_session.preview.state.pending_object_modifier == Some('i')
            {
                select_inner_word(app);
                app.preview_session.preview.state.pending_object_modifier = None;
            } else {
                move_word_forward(app, count);
            }
        }
        _ => {}
    }
}

struct PreviewNormalTarget<'a> {
    app: &'a mut App,
}

struct PreviewOperatorTarget<'a> {
    app: &'a mut App,
}

impl PendingOperatorTarget for PreviewOperatorTarget<'_> {
    fn set_modifier(&mut self, modifier: char) -> bool {
        if modifier == 'i' {
            self.app
                .preview_session
                .preview
                .state
                .pending_object_modifier = Some(modifier);
            true
        } else {
            false
        }
    }

    fn repeat_operator(&mut self, op: char, count: usize) -> bool {
        match op {
            'd' => {
                delete_line(self.app, count);
                true
            }
            'c' => {
                delete_line_content(self.app);
                self.app.preview_session.preview.state.mode = InputMode::Insert;
                true
            }
            'y' => {
                yank_line(self.app);
                self.app.preview_session.preview.state.status_message =
                    Some(("Yanked line".to_string(), std::time::Instant::now()));
                true
            }
            'g' => {
                move_to_start_of_file(self.app, count);
                true
            }
            _ => false,
        }
    }

    fn apply_motion(
        &mut self,
        _op: char,
        _motion: super::command_parser::OperatorMotion,
        _count: usize,
    ) -> bool {
        false
    }

    fn clear_pending(&mut self) {
        self.app.preview_session.preview.state.pending_operator = None;
        self.app
            .preview_session
            .preview
            .state
            .pending_object_modifier = None;
    }
}

impl CommonActionTarget for PreviewNormalTarget<'_> {
    fn move_left(&mut self, count: usize) {
        move_left(self.app, count);
    }

    fn move_right(&mut self, count: usize) {
        move_right(self.app, count);
    }

    fn move_start_of_line(&mut self) {
        move_start_of_line(self.app);
    }

    fn move_end_of_line(&mut self) {
        move_end_of_line(self.app);
    }

    fn move_first_non_blank(&mut self) {
        move_first_non_blank(self.app);
    }

    fn move_word_forward(&mut self, count: usize) {
        move_word_forward(self.app, count);
    }

    fn move_word_end_forward(&mut self, count: usize) {
        move_word_end_forward(self.app, count);
    }

    fn move_word_backward(&mut self, count: usize) {
        move_word_backward(self.app, count);
    }

    fn move_big_word_forward(&mut self, count: usize) {
        move_big_word_forward(self.app, count);
    }

    fn move_big_word_backward(&mut self, count: usize) {
        move_big_word_backward(self.app, count);
    }

    fn enter_insert_before(&mut self) {
        self.app.preview_session.preview.state.mode = InputMode::Insert;
    }

    fn enter_insert_after(&mut self) {
        move_right(self.app, 1);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
    }

    fn enter_insert_at_end(&mut self) {
        move_end_of_line_for_insert(self.app);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
    }

    fn enter_insert_at_first_non_blank(&mut self) {
        move_first_non_blank(self.app);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
    }

    fn delete_char_under_cursor(&mut self, count: usize) -> bool {
        for _ in 0..count {
            delete_char_under_cursor(self.app);
        }
        true
    }

    fn delete_to_end_of_line(&mut self) -> bool {
        delete_to_end_of_line(self.app);
        true
    }

    fn change_to_end_of_line(&mut self) -> bool {
        delete_to_end_of_line(self.app);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
        true
    }

    fn substitute_char(&mut self) -> bool {
        delete_char_under_cursor(self.app);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
        true
    }

    fn substitute_line(&mut self) -> bool {
        delete_line_content(self.app);
        self.app.preview_session.preview.state.mode = InputMode::Insert;
        true
    }

    fn start_operator(&mut self, op: char) {
        self.app.preview_session.preview.state.pending_operator = Some(op);
    }
}
