use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorMotion {
    StartOfLine,
    EndOfLine,
    WordForward,
    WordEndForward,
    WordBackward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperatorIntent {
    Cancel,
    SetModifier(char),
    RepeatOperator,
    Motion(OperatorMotion),
}

pub fn push_count_digit(buffer: &mut String, key: KeyEvent) -> bool {
    let KeyCode::Char(ch) = key.code else {
        return false;
    };
    if !key.modifiers.is_empty() || !ch.is_ascii_digit() {
        return false;
    }
    if ch == '0' && buffer.is_empty() {
        return false;
    }

    buffer.push(ch);
    true
}

pub fn take_count(buffer: &mut String) -> usize {
    if buffer.is_empty() {
        return 1;
    }

    let count = buffer.parse::<usize>().unwrap_or(1);
    buffer.clear();
    count.max(1)
}

pub fn parse_pending_operator_intent(key: KeyEvent, op: char) -> Option<PendingOperatorIntent> {
    match key.code {
        KeyCode::Esc => Some(PendingOperatorIntent::Cancel),
        KeyCode::Char('i') | KeyCode::Char('a') => {
            if let KeyCode::Char(modifier) = key.code {
                Some(PendingOperatorIntent::SetModifier(modifier))
            } else {
                None
            }
        }
        KeyCode::Char(ch) if ch == op => Some(PendingOperatorIntent::RepeatOperator),
        KeyCode::Char('w') => Some(PendingOperatorIntent::Motion(OperatorMotion::WordForward)),
        KeyCode::Char('e') => Some(PendingOperatorIntent::Motion(
            OperatorMotion::WordEndForward,
        )),
        KeyCode::Char('b') => Some(PendingOperatorIntent::Motion(OperatorMotion::WordBackward)),
        KeyCode::Char('$') => Some(PendingOperatorIntent::Motion(OperatorMotion::EndOfLine)),
        KeyCode::Char('0') => Some(PendingOperatorIntent::Motion(OperatorMotion::StartOfLine)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_pending_operator_intent, push_count_digit, take_count, OperatorMotion,
        PendingOperatorIntent,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn count_parser_accumulates_digits_but_not_leading_zero() {
        let mut buffer = String::new();

        assert!(!push_count_digit(
            &mut buffer,
            KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)
        ));
        assert!(push_count_digit(
            &mut buffer,
            KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)
        ));
        assert!(push_count_digit(
            &mut buffer,
            KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE)
        ));

        assert_eq!(buffer, "25");
        assert_eq!(take_count(&mut buffer), 25);
        assert!(buffer.is_empty());
    }

    #[test]
    fn pending_operator_parser_maps_repeat_and_motion_keys() {
        assert_eq!(
            parse_pending_operator_intent(
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
                'd'
            ),
            Some(PendingOperatorIntent::RepeatOperator)
        );
        assert_eq!(
            parse_pending_operator_intent(
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
                'd'
            ),
            Some(PendingOperatorIntent::Motion(OperatorMotion::WordForward))
        );
        assert_eq!(
            parse_pending_operator_intent(
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                'c'
            ),
            Some(PendingOperatorIntent::SetModifier('i'))
        );
    }
}
