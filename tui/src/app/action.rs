use crate::preview::structured_log::LogEntry;
use crate::preview::{PreviewContent, PreviewSource};
use crate::runtime::interactive::input::InputEvent;
use crate::search::matcher::MatcherState;
use crate::search::types::SearchMode;

pub enum AppEvent {
    Input(InputEvent),
    Matcher(MatcherState, u64),
    Preview(PreviewSource, PreviewContent),
    LogAppend(String, Vec<LogEntry>),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    Search,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Visual,
    VisualLine,
    Insert,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTab {
    Overview,
    Search,
    Preview,
    Logs,
    Layout,
}

impl HelpTab {
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Search => "Search",
            Self::Preview => "Preview",
            Self::Logs => "Logs",
            Self::Layout => "Layout",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Overview => Self::Search,
            Self::Search => Self::Preview,
            Self::Preview => Self::Logs,
            Self::Logs => Self::Layout,
            Self::Layout => Self::Overview,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Overview => Self::Layout,
            Self::Search => Self::Overview,
            Self::Preview => Self::Search,
            Self::Logs => Self::Preview,
            Self::Layout => Self::Logs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Quit,
    FocusSearch,
    FocusPreview,
    ToggleHelp,
    CloseHelp,
    ShowHelpTab(HelpTab),
    NextHelpTab,
    PreviousHelpTab,
    TogglePreviewFullscreen,
    SwapPanes,
    AdjustPreviewWidth(i16),
    ToggleSearchBarPosition,
    TogglePreviewVisibility,
    ToggleExactMatcher,
    SetSearchMode(SearchMode),
    RequestSearchRestart,
}
