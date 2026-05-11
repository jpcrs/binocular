use super::command_parser::{parse_pending_operator_intent, OperatorMotion, PendingOperatorIntent};
use crossterm::event::KeyEvent;

pub enum PendingOperatorResult {
    Unhandled,
    AwaitingMore,
    Applied { changed: bool },
    Cleared,
}

pub trait PendingOperatorTarget {
    fn set_modifier(&mut self, modifier: char) -> bool;
    fn repeat_operator(&mut self, op: char, count: usize) -> bool;
    fn apply_motion(&mut self, op: char, motion: OperatorMotion, count: usize) -> bool;
    fn clear_pending(&mut self);
}

pub fn handle_pending_operator<T: PendingOperatorTarget>(
    target: &mut T,
    key: KeyEvent,
    op: char,
    count: usize,
) -> PendingOperatorResult {
    let Some(intent) = parse_pending_operator_intent(key, op) else {
        return PendingOperatorResult::Unhandled;
    };

    match intent {
        PendingOperatorIntent::Cancel => {
            target.clear_pending();
            PendingOperatorResult::Cleared
        }
        PendingOperatorIntent::SetModifier(modifier) => {
            if target.set_modifier(modifier) {
                PendingOperatorResult::AwaitingMore
            } else {
                target.clear_pending();
                PendingOperatorResult::Cleared
            }
        }
        PendingOperatorIntent::RepeatOperator => {
            let changed = target.repeat_operator(op, count);
            target.clear_pending();
            PendingOperatorResult::Applied { changed }
        }
        PendingOperatorIntent::Motion(motion) => {
            let changed = target.apply_motion(op, motion, count);
            target.clear_pending();
            PendingOperatorResult::Applied { changed }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{handle_pending_operator, PendingOperatorResult, PendingOperatorTarget};
    use crate::input::vim::command_parser::OperatorMotion;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[derive(Default)]
    struct FakeTarget {
        modifier: Option<char>,
        cleared: bool,
        repeated: Option<(char, usize)>,
        motion: Option<(char, OperatorMotion, usize)>,
    }

    impl PendingOperatorTarget for FakeTarget {
        fn set_modifier(&mut self, modifier: char) -> bool {
            self.modifier = Some(modifier);
            true
        }

        fn repeat_operator(&mut self, op: char, count: usize) -> bool {
            self.repeated = Some((op, count));
            true
        }

        fn apply_motion(&mut self, op: char, motion: OperatorMotion, count: usize) -> bool {
            self.motion = Some((op, motion, count));
            true
        }

        fn clear_pending(&mut self) {
            self.cleared = true;
        }
    }

    #[test]
    fn shared_handler_tracks_modifier_repeat_and_motion() {
        let mut target = FakeTarget::default();
        assert!(matches!(
            handle_pending_operator(
                &mut target,
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                'd',
                1
            ),
            PendingOperatorResult::AwaitingMore
        ));
        assert_eq!(target.modifier, Some('i'));

        assert!(matches!(
            handle_pending_operator(
                &mut target,
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
                'd',
                2
            ),
            PendingOperatorResult::Applied { changed: true }
        ));
        assert_eq!(target.repeated, Some(('d', 2)));

        assert!(matches!(
            handle_pending_operator(
                &mut target,
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
                'c',
                3
            ),
            PendingOperatorResult::Applied { changed: true }
        ));
        assert_eq!(target.motion, Some(('c', OperatorMotion::WordForward, 3)));
    }
}
