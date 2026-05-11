use super::common_actions::CommonNormalAction;

pub trait CommonActionTarget {
    fn move_left(&mut self, count: usize);
    fn move_right(&mut self, count: usize);
    fn move_start_of_line(&mut self);
    fn move_end_of_line(&mut self);
    fn move_first_non_blank(&mut self);
    fn move_word_forward(&mut self, count: usize);
    fn move_word_end_forward(&mut self, count: usize);
    fn move_word_backward(&mut self, count: usize);
    fn move_big_word_forward(&mut self, count: usize);
    fn move_big_word_backward(&mut self, count: usize);
    fn enter_insert_before(&mut self);
    fn enter_insert_after(&mut self);
    fn enter_insert_at_end(&mut self);
    fn enter_insert_at_first_non_blank(&mut self);
    fn delete_char_under_cursor(&mut self, count: usize) -> bool;
    fn delete_to_end_of_line(&mut self) -> bool;
    fn change_to_end_of_line(&mut self) -> bool;
    fn substitute_char(&mut self) -> bool;
    fn substitute_line(&mut self) -> bool;
    fn start_operator(&mut self, op: char);
}

pub fn apply_common_normal_action<T: CommonActionTarget>(
    target: &mut T,
    action: CommonNormalAction,
    count: usize,
) -> bool {
    match action {
        CommonNormalAction::MoveLeft => target.move_left(count),
        CommonNormalAction::MoveRight => target.move_right(count),
        CommonNormalAction::MoveStartOfLine => target.move_start_of_line(),
        CommonNormalAction::MoveEndOfLine => target.move_end_of_line(),
        CommonNormalAction::MoveFirstNonBlank => target.move_first_non_blank(),
        CommonNormalAction::MoveWordForward => target.move_word_forward(count),
        CommonNormalAction::MoveWordEndForward => target.move_word_end_forward(count),
        CommonNormalAction::MoveWordBackward => target.move_word_backward(count),
        CommonNormalAction::MoveBigWordForward => target.move_big_word_forward(count),
        CommonNormalAction::MoveBigWordBackward => target.move_big_word_backward(count),
        CommonNormalAction::EnterInsertBefore => target.enter_insert_before(),
        CommonNormalAction::EnterInsertAfter => target.enter_insert_after(),
        CommonNormalAction::EnterInsertAtEnd => target.enter_insert_at_end(),
        CommonNormalAction::EnterInsertAtFirstNonBlank => target.enter_insert_at_first_non_blank(),
        CommonNormalAction::DeleteCharUnderCursor => {
            return target.delete_char_under_cursor(count);
        }
        CommonNormalAction::DeleteToEndOfLine => {
            return target.delete_to_end_of_line();
        }
        CommonNormalAction::ChangeToEndOfLine => {
            return target.change_to_end_of_line();
        }
        CommonNormalAction::SubstituteChar => {
            return target.substitute_char();
        }
        CommonNormalAction::SubstituteLine => {
            return target.substitute_line();
        }
        CommonNormalAction::StartDeleteOperator => target.start_operator('d'),
        CommonNormalAction::StartChangeOperator => target.start_operator('c'),
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{apply_common_normal_action, CommonActionTarget};
    use crate::input::vim::common_actions::CommonNormalAction;

    #[derive(Default)]
    struct FakeTarget {
        moved_left: usize,
        last_operator: Option<char>,
        changed: bool,
    }

    impl CommonActionTarget for FakeTarget {
        fn move_left(&mut self, count: usize) {
            self.moved_left += count;
        }

        fn move_right(&mut self, _count: usize) {}
        fn move_start_of_line(&mut self) {}
        fn move_end_of_line(&mut self) {}
        fn move_first_non_blank(&mut self) {}
        fn move_word_forward(&mut self, _count: usize) {}
        fn move_word_end_forward(&mut self, _count: usize) {}
        fn move_word_backward(&mut self, _count: usize) {}
        fn move_big_word_forward(&mut self, _count: usize) {}
        fn move_big_word_backward(&mut self, _count: usize) {}
        fn enter_insert_before(&mut self) {}
        fn enter_insert_after(&mut self) {}
        fn enter_insert_at_end(&mut self) {}
        fn enter_insert_at_first_non_blank(&mut self) {}

        fn delete_char_under_cursor(&mut self, _count: usize) -> bool {
            self.changed = true;
            true
        }

        fn delete_to_end_of_line(&mut self) -> bool {
            self.changed = true;
            true
        }

        fn change_to_end_of_line(&mut self) -> bool {
            self.changed = true;
            true
        }

        fn substitute_char(&mut self) -> bool {
            self.changed = true;
            true
        }

        fn substitute_line(&mut self) -> bool {
            self.changed = true;
            true
        }

        fn start_operator(&mut self, op: char) {
            self.last_operator = Some(op);
        }
    }

    #[test]
    fn applies_motion_actions_through_target() {
        let mut target = FakeTarget::default();
        let changed = apply_common_normal_action(&mut target, CommonNormalAction::MoveLeft, 3);

        assert!(!changed);
        assert_eq!(target.moved_left, 3);
    }

    #[test]
    fn returns_change_state_for_edit_actions() {
        let mut target = FakeTarget::default();
        let changed =
            apply_common_normal_action(&mut target, CommonNormalAction::DeleteCharUnderCursor, 2);

        assert!(changed);
        assert!(target.changed);
    }

    #[test]
    fn starts_operators_through_target() {
        let mut target = FakeTarget::default();
        let changed =
            apply_common_normal_action(&mut target, CommonNormalAction::StartChangeOperator, 1);

        assert!(!changed);
        assert_eq!(target.last_operator, Some('c'));
    }
}
