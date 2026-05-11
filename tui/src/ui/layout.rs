use crate::app::LayoutState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct UiAreas {
    pub search_results: Rect,
    pub preview: Option<Rect>,
    pub search_bar: Rect,
    pub show_search: bool,
}

pub fn split_main_layout(area: Rect, show_preview: bool, layout: &LayoutState) -> UiAreas {
    if show_preview && layout.preview_fullscreen {
        return UiAreas {
            search_bar: Rect::default(),
            search_results: Rect::default(),
            preview: Some(area),
            show_search: false,
        };
    }

    if show_preview {
        let results_pct = 100 - layout.preview_percent;
        let preview_pct = layout.preview_percent;

        let (left_pct, right_pct) = if layout.panes_swapped {
            (preview_pct, results_pct)
        } else {
            (results_pct, preview_pct)
        };

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(left_pct),
                Constraint::Percentage(right_pct),
            ])
            .split(area);

        let (results_col, preview_col) = if layout.panes_swapped {
            (columns[1], columns[0])
        } else {
            (columns[0], columns[1])
        };

        let (search_bar, search_results) =
            split_results_column(results_col, layout.search_bar_at_bottom);

        UiAreas {
            search_bar,
            search_results,
            preview: Some(preview_col),
            show_search: true,
        }
    } else {
        let (search_bar, search_results) = split_results_column(area, layout.search_bar_at_bottom);

        UiAreas {
            search_bar,
            search_results,
            preview: None,
            show_search: true,
        }
    }
}

fn split_results_column(col: Rect, bar_at_bottom: bool) -> (Rect, Rect) {
    let parts = if bar_at_bottom {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(col)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(col)
    };

    if bar_at_bottom {
        (parts[1], parts[0])
    } else {
        (parts[0], parts[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_preview_hides_search_regions() {
        let layout = LayoutState {
            preview_fullscreen: true,
            ..LayoutState::default()
        };
        let area = Rect::new(0, 0, 120, 40);

        let areas = split_main_layout(area, true, &layout);

        assert!(!areas.show_search);
        assert_eq!(areas.preview, Some(area));
    }

    #[test]
    fn hidden_preview_uses_single_search_column() {
        let area = Rect::new(0, 0, 120, 40);
        let areas = split_main_layout(area, false, &LayoutState::default());

        assert!(areas.show_search);
        assert!(areas.preview.is_none());
        assert_eq!(areas.search_bar.height, 3);
        assert_eq!(areas.search_results.height, 37);
    }

    #[test]
    fn swapped_panes_move_preview_to_left_column() {
        let layout = LayoutState {
            panes_swapped: true,
            preview_percent: 40,
            ..LayoutState::default()
        };
        let area = Rect::new(0, 0, 100, 30);

        let areas = split_main_layout(area, true, &layout);
        let preview = areas.preview.expect("preview area");

        assert_eq!(preview.x, 0);
        assert_eq!(preview.width, 40);
        assert_eq!(areas.search_results.x, 40);
    }

    #[test]
    fn search_bar_can_move_to_bottom() {
        let layout = LayoutState {
            search_bar_at_bottom: true,
            ..LayoutState::default()
        };
        let area = Rect::new(0, 0, 80, 20);

        let areas = split_main_layout(area, false, &layout);

        assert_eq!(areas.search_bar.y, 17);
        assert_eq!(areas.search_bar.height, 3);
        assert_eq!(areas.search_results.y, 0);
    }
}
