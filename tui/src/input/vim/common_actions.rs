use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonNormalAction {
    MoveLeft,
    MoveRight,
    MoveStartOfLine,
    MoveEndOfLine,
    MoveFirstNonBlank,
    MoveWordForward,
    MoveWordEndForward,
    MoveWordBackward,
    MoveBigWordForward,
    MoveBigWordBackward,
    EnterInsertBefore,
    EnterInsertAfter,
    EnterInsertAtEnd,
    EnterInsertAtFirstNonBlank,
    DeleteCharUnderCursor,
    DeleteToEndOfLine,
    ChangeToEndOfLine,
    SubstituteChar,
    SubstituteLine,
    StartDeleteOperator,
    StartChangeOperator,
}

pub fn parse_common_normal_action(key: KeyEvent) -> Option<CommonNormalAction> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    match key.code {
        KeyCode::Char('h') | KeyCode::Left => Some(CommonNormalAction::MoveLeft),
        KeyCode::Char('l') | KeyCode::Right => Some(CommonNormalAction::MoveRight),
        KeyCode::Char('H') => Some(CommonNormalAction::MoveStartOfLine),
        KeyCode::Char('L') => Some(CommonNormalAction::MoveEndOfLine),
        KeyCode::Char('0') => Some(CommonNormalAction::MoveStartOfLine),
        KeyCode::Char('$') => Some(CommonNormalAction::MoveEndOfLine),
        KeyCode::Char('^') => Some(CommonNormalAction::MoveFirstNonBlank),
        KeyCode::Char('w') => Some(CommonNormalAction::MoveWordForward),
        KeyCode::Char('e') => Some(CommonNormalAction::MoveWordEndForward),
        KeyCode::Char('b') => Some(CommonNormalAction::MoveWordBackward),
        KeyCode::Char('W') => Some(CommonNormalAction::MoveBigWordForward),
        KeyCode::Char('B') => Some(CommonNormalAction::MoveBigWordBackward),
        KeyCode::Char('i') => Some(CommonNormalAction::EnterInsertBefore),
        KeyCode::Char('a') => Some(CommonNormalAction::EnterInsertAfter),
        KeyCode::Char('A') => Some(CommonNormalAction::EnterInsertAtEnd),
        KeyCode::Char('I') => Some(CommonNormalAction::EnterInsertAtFirstNonBlank),
        KeyCode::Char('x') => Some(CommonNormalAction::DeleteCharUnderCursor),
        KeyCode::Char('D') => Some(CommonNormalAction::DeleteToEndOfLine),
        KeyCode::Char('C') => Some(CommonNormalAction::ChangeToEndOfLine),
        KeyCode::Char('s') => Some(CommonNormalAction::SubstituteChar),
        KeyCode::Char('S') => Some(CommonNormalAction::SubstituteLine),
        KeyCode::Char('d') => Some(CommonNormalAction::StartDeleteOperator),
        KeyCode::Char('c') => Some(CommonNormalAction::StartChangeOperator),
        _ => None,
    }
}
