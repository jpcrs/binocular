use crate::app::{App, AppAction, HelpTab};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_help_modal_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.apply_action(AppAction::CloseHelp),
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            app.apply_action(AppAction::NextHelpTab)
        }
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
            app.apply_action(AppAction::PreviousHelpTab)
        }
        KeyCode::Char('1') => app.apply_action(AppAction::ShowHelpTab(HelpTab::Overview)),
        KeyCode::Char('2') => app.apply_action(AppAction::ShowHelpTab(HelpTab::Search)),
        KeyCode::Char('3') => app.apply_action(AppAction::ShowHelpTab(HelpTab::Preview)),
        KeyCode::Char('4') => app.apply_action(AppAction::ShowHelpTab(HelpTab::Logs)),
        KeyCode::Char('5') => app.apply_action(AppAction::ShowHelpTab(HelpTab::Layout)),
        _ => {}
    }
}
