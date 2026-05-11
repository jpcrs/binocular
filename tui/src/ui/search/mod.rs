mod bar;
mod results;

use crate::app::{InputMode, Mode};
use ratatui::style::{Color, Style};

pub use bar::{render_search_bar, SearchBarView};
pub use results::{render_search_results, SearchResultsView};

pub(super) fn search_border_style(app_mode: Mode, query_mode: InputMode) -> Style {
    if app_mode != Mode::Search {
        return Style::default();
    }

    if query_mode == InputMode::Insert {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Blue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn search_mode_insert_uses_green_border() {
        let style = search_border_style(Mode::Search, InputMode::Insert);
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn preview_mode_results_use_default_border() {
        let style = search_border_style(Mode::Preview, InputMode::Normal);
        assert_eq!(style.fg, None);
    }
}
