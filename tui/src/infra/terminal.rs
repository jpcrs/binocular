use crossterm::{
    cursor::SetCursorStyle,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

pub struct TerminalSessionGuard {
    cursor_style_is_bar: bool,
}

impl TerminalSessionGuard {
    pub fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stderr(),
            EnterAlternateScreen,
            SetCursorStyle::BlinkingBlock
        )?;
        Ok(Self {
            cursor_style_is_bar: false,
        })
    }

    pub fn sync_cursor_style(&mut self, should_be_bar: bool) {
        if should_be_bar == self.cursor_style_is_bar {
            return;
        }

        let style = if should_be_bar {
            SetCursorStyle::BlinkingBar
        } else {
            SetCursorStyle::BlinkingBlock
        };
        let _ = execute!(io::stderr(), style);
        self.cursor_style_is_bar = should_be_bar;
    }
}

impl Drop for TerminalSessionGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stderr(), SetCursorStyle::BlinkingBlock);
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
