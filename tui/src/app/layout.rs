use super::HelpTab;

pub struct HelpState {
    pub visible: bool,
    pub tab: HelpTab,
}

pub struct LayoutState {
    pub preview_fullscreen: bool,
    pub panes_swapped: bool,
    pub preview_percent: u16,
    pub search_bar_at_bottom: bool,
    pub preview_hidden: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            preview_fullscreen: false,
            panes_swapped: false,
            preview_percent: 50,
            search_bar_at_bottom: false,
            preview_hidden: false,
        }
    }
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            visible: false,
            tab: HelpTab::Overview,
        }
    }
}

#[derive(Default)]
pub(crate) struct ViewportMetrics {
    pub(crate) terminal_width: u16,
    pub(crate) terminal_height: u16,
    pub(crate) preview_width: u16,
    pub(crate) preview_height: u16,
}
