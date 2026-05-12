use super::{AppAction, HelpState, HelpTab, InputMode, LayoutState, Mode};
use crate::config::{Keybindings, LoadedAppConfig};
use crate::output::SelectionOutput;
use crate::preview::rich_text::TextUndoFrame;
use crate::preview::{PreviewContent, PreviewSource};
use crate::runtime::config::RunConfig;
use crate::search::types::{
    MatcherMode, SearchConfig, SearchItem, SearchMode, SearchResult, SearchSettings,
};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use std::collections::HashMap;

use super::layout::ViewportMetrics;

pub struct PreviewState {
    pub scroll: usize,
    pub scroll_char: usize,
    pub highlight_line: Option<usize>,
    pub cursor_line: usize,
    pub cursor_char: usize,
    pub search_query: String,
    pub search_active: bool,
    pub input_buffer: String,
    pub command_buffer: String,
    pub waiting_for_char_search: Option<(bool, usize)>,
    pub last_char_search: Option<(char, bool)>,
    pub mode: InputMode,
    pub selection_start: Option<(usize, usize)>,
    pub pending_object_modifier: Option<char>,
    pub pending_operator: Option<char>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub undo_stack: Vec<TextUndoFrame>,
    pub redo_stack: Vec<TextUndoFrame>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            scroll: 0,
            scroll_char: 0,
            highlight_line: None,
            cursor_line: 0,
            cursor_char: 0,
            search_query: String::new(),
            search_active: false,
            input_buffer: String::new(),
            command_buffer: String::new(),
            waiting_for_char_search: None,
            last_char_search: None,
            mode: InputMode::Normal,
            selection_start: None,
            pending_object_modifier: None,
            pending_operator: None,
            status_message: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

pub struct Query {
    pub text: String,
    /// Cursor position (char index) in the search query.
    pub cursor: usize,
    /// Count prefix buffer for normal-mode commands (e.g. `3w`, `2x`).
    pub count_buffer: String,
    /// Vim mode for the search bar (Insert or Normal).
    pub mode: InputMode,
    /// Pending operator for search bar vim commands (e.g., 'd', 'c', 'y').
    pub pending_op: Option<char>,
    /// Pending text object modifier (e.g., 'i' for inner, 'a' for around).
    pub pending_modifier: Option<char>,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            count_buffer: String::new(),
            mode: InputMode::Insert,
            pending_op: None,
            pending_modifier: None,
        }
    }
}

pub struct Search {
    pub results: Vec<SearchResult>,
    pub total_matches: u64,
    pub total_items: u64,
    pub selection: usize,
    pub selected_item: Option<SearchItem>,
    pub marked_items: HashMap<SearchItem, Option<usize>>,
    pub diff_marked_items: std::collections::HashSet<SearchItem>,
    pub scroll_state: ListState,
    pub working: bool,
}

impl Default for Search {
    fn default() -> Self {
        let mut scroll_state = ListState::default();
        scroll_state.select(Some(0));
        Self {
            results: Vec::new(),
            total_matches: 0,
            total_items: 0,
            selection: 0,
            selected_item: None,
            marked_items: HashMap::new(),
            diff_marked_items: std::collections::HashSet::new(),
            scroll_state,
            working: false,
        }
    }
}

impl Search {
    pub fn update_selection(&mut self) {
        if self.results.is_empty() {
            self.selection = 0;
            self.selected_item = None;
            self.scroll_state.select(None);
            return;
        }

        if let Some(ref selected) = self.selected_item {
            if let Some(new_idx) = self
                .results
                .iter()
                .position(|result| &result.item == selected)
            {
                self.selection = new_idx;
                self.scroll_state.select(Some(new_idx));
                return;
            }
        }

        if self.selection >= self.results.len() {
            self.selection = self.results.len().saturating_sub(1);
        }
        self.scroll_state.select(Some(self.selection));

        if let Some(result) = self.results.get(self.selection) {
            self.selected_item = Some(result.item.clone());
        }
    }

    pub fn next(&mut self) {
        if self.total_matches > 0 && self.selection < self.total_matches as usize - 1 {
            self.selection += 1;
            self.scroll_state.select(Some(self.selection));
            if let Some(result) = self.results.get(self.selection) {
                self.selected_item = Some(result.item.clone());
            }
        }
    }

    pub fn previous(&mut self) {
        if self.selection > 0 {
            self.selection -= 1;
            self.scroll_state.select(Some(self.selection));
            if let Some(result) = self.results.get(self.selection) {
                self.selected_item = Some(result.item.clone());
            }
        }
    }
}

pub struct Preview {
    pub content: Option<PreviewContent>,
    pub source: Option<PreviewSource>,
    pub state: PreviewState,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            content: None,
            source: None,
            state: PreviewState::default(),
        }
    }
}

pub struct RuntimeConfig {
    pub run: RunConfig,
    pub search: SearchConfig,
    pub app_config: LoadedAppConfig,
}

pub struct SearchSessionState {
    pub settings: SearchSettings,
    pub query: Query,
    pub search: Search,
}

pub struct PreviewSessionState {
    pub preview: Preview,
}

pub struct UiState {
    pub help: HelpState,
    pub layout: LayoutState,
    pub mode: Mode,
    pub should_quit: bool,
    pub restart_search: bool,
    pub(crate) viewport: ViewportMetrics,
}

pub struct App {
    pub runtime: RuntimeConfig,
    pub search_session: SearchSessionState,
    pub preview_session: PreviewSessionState,
    pub ui: UiState,
    selected_output: Vec<SelectionOutput>,
}

impl App {
    pub fn from_configs(
        run_config: RunConfig,
        search_config: SearchConfig,
        app_config: LoadedAppConfig,
    ) -> Self {
        let log_mode = run_config.log;
        let direct_diff_mode = run_config.diff.is_some();
        let search_settings = search_config.settings;
        let query = if let Some(ref initial) = search_config.query {
            let len = initial.chars().count();
            Query {
                text: initial.clone(),
                cursor: len,
                ..Query::default()
            }
        } else {
            Query::default()
        };
        Self {
            runtime: RuntimeConfig {
                run: run_config,
                search: search_config,
                app_config,
            },
            search_session: SearchSessionState {
                settings: search_settings,
                query,
                search: Search::default(),
            },
            preview_session: PreviewSessionState {
                preview: Preview::default(),
            },
            ui: UiState {
                help: HelpState::default(),
                layout: LayoutState::default(),
                should_quit: false,
                mode: if log_mode || direct_diff_mode {
                    Mode::Preview
                } else {
                    Mode::Search
                },
                restart_search: false,
                viewport: ViewportMetrics::default(),
            },
            selected_output: Vec::new(),
        }
    }

    pub fn apply_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => self.ui.should_quit = true,
            AppAction::FocusSearch => self.ui.mode = Mode::Search,
            AppAction::FocusPreview => self.ui.mode = Mode::Preview,
            AppAction::ToggleHelp => {
                self.ui.help.visible = !self.ui.help.visible;
                if self.ui.help.visible {
                    self.ui.help.tab = if self.ui.mode == Mode::Preview {
                        HelpTab::Preview
                    } else {
                        HelpTab::Overview
                    };
                }
            }
            AppAction::CloseHelp => self.ui.help.visible = false,
            AppAction::ShowHelpTab(tab) => self.ui.help.tab = tab,
            AppAction::NextHelpTab => self.ui.help.tab = self.ui.help.tab.next(),
            AppAction::PreviousHelpTab => self.ui.help.tab = self.ui.help.tab.previous(),
            AppAction::TogglePreviewFullscreen => {
                self.ui.layout.preview_fullscreen = !self.ui.layout.preview_fullscreen;
                if self.ui.layout.preview_fullscreen && self.ui.mode == Mode::Search {
                    self.ui.mode = Mode::Preview;
                }
            }
            AppAction::SwapPanes => {
                self.ui.layout.panes_swapped = !self.ui.layout.panes_swapped;
            }
            AppAction::AdjustPreviewWidth(delta) => {
                if delta.is_positive() {
                    self.ui.layout.preview_percent =
                        (self.ui.layout.preview_percent + delta as u16).min(80);
                } else {
                    self.ui.layout.preview_percent = self
                        .ui
                        .layout
                        .preview_percent
                        .saturating_sub(delta.unsigned_abs())
                        .max(20);
                }
            }
            AppAction::ToggleSearchBarPosition => {
                self.ui.layout.search_bar_at_bottom = !self.ui.layout.search_bar_at_bottom;
            }
            AppAction::TogglePreviewVisibility => {
                self.ui.layout.preview_hidden = !self.ui.layout.preview_hidden;
                if self.ui.layout.preview_hidden && self.ui.mode == Mode::Preview {
                    self.ui.mode = Mode::Search;
                }
            }
            AppAction::ToggleExactMatcher => {
                self.search_session.settings.matcher =
                    self.search_session.settings.matcher.toggle();
                self.ui.restart_search = true;
            }
            AppAction::SetSearchMode(mode) => {
                self.search_session.settings.mode = mode;
                self.ui.restart_search = true;
            }
            AppAction::RequestSearchRestart => self.ui.restart_search = true,
        }
    }

    pub fn show_preview(&self) -> bool {
        if self.runtime.run.diff.is_some() || self.runtime.search.git_search_scope.is_some() {
            return true;
        }
        let naturally_visible = !self.runtime.run.stdin || self.runtime.run.has_preview_command();
        naturally_visible && !self.ui.layout.preview_hidden
    }

    pub fn keybindings(&self) -> &Keybindings {
        &self.runtime.app_config.keybindings
    }

    pub fn log_max_entries(&self) -> usize {
        self.runtime.app_config.log.max_entries
    }

    pub fn is_content_mode(&self) -> bool {
        self.search_session.settings.mode == SearchMode::Grep
            || self.search_session.settings.mode == SearchMode::GitHistory
    }

    pub fn is_git_mode(&self) -> bool {
        matches!(
            self.search_session.settings.mode,
            SearchMode::GitHistory | SearchMode::GitBranches | SearchMode::GitCommits
        )
    }

    pub fn is_dir_mode(&self) -> bool {
        self.search_session.settings.mode == SearchMode::Dirs
    }

    pub fn is_file_name_mode(&self) -> bool {
        self.search_session.settings.mode == SearchMode::Files
    }

    pub fn is_exact_mode(&self) -> bool {
        self.search_session.settings.matcher == MatcherMode::Exact
    }

    pub fn search_config(&self) -> SearchConfig {
        let mut search_config = self
            .runtime
            .search
            .with_settings(self.search_session.settings);
        search_config.query = Some(self.search_session.query.text.clone());
        search_config
    }

    pub fn preview_file_path(&self) -> Option<&str> {
        self.preview_session
            .preview
            .source
            .as_ref()
            .and_then(PreviewSource::file_path)
    }

    pub fn set_selected_output(&mut self, output: Vec<SelectionOutput>) {
        self.selected_output = output;
    }

    pub fn take_selected_output(&mut self) -> Vec<SelectionOutput> {
        std::mem::take(&mut self.selected_output)
    }

    pub fn set_terminal_area(&mut self, area: Rect) {
        self.ui.viewport.terminal_width = area.width;
        self.ui.viewport.terminal_height = area.height;
    }

    pub fn set_terminal_size(&mut self, width: u16, height: u16) {
        self.ui.viewport.terminal_width = width;
        self.ui.viewport.terminal_height = height;
    }

    pub fn refresh_viewports(&mut self) {
        let (preview_width, preview_height) = if self.runtime.run.log
            || self.runtime.run.diff.is_some()
            || self.runtime.search.git_search_scope.is_some()
        {
            (
                self.ui.viewport.terminal_width,
                self.ui.viewport.terminal_height,
            )
        } else if !self.show_preview() {
            (0, 0)
        } else if self.ui.layout.preview_fullscreen {
            (
                self.ui.viewport.terminal_width,
                self.ui.viewport.terminal_height,
            )
        } else {
            (
                self.ui.viewport.terminal_width * self.ui.layout.preview_percent / 100,
                self.ui.viewport.terminal_height,
            )
        };

        self.ui.viewport.preview_width = preview_width;
        self.ui.viewport.preview_height = preview_height;
    }

    pub fn preview_width(&self) -> u16 {
        self.ui.viewport.preview_width
    }

    pub fn preview_height(&self) -> u16 {
        self.ui.viewport.preview_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::OutputFormat;
    use crate::config::LoadedAppConfig;
    use crate::runtime::config::RunConfig;

    fn run_config() -> RunConfig {
        RunConfig {
            headless: false,
            output_format: OutputFormat::Plain,
            output_file: None,
            stdin: false,
            log: false,
            diff: None,
            preview_command: None,
            preview_delimiter: ":".to_string(),
            split: None,
            log_files: Vec::new(),
        }
    }

    fn search_config() -> SearchConfig {
        SearchConfig {
            query: None,
            locations: vec![],
            search_pdf: false,
            no_hidden: false,
            no_git_ignore: false,
            no_ignore: false,
            no_default_ignore_dirs: false,
            git_search_scope: None,
            settings: SearchSettings {
                mode: SearchMode::Path,
                matcher: MatcherMode::Fuzzy,
            },
        }
    }

    #[test]
    fn file_mode_starts_in_search_with_preview_visible() {
        let app = App::from_configs(run_config(), search_config(), LoadedAppConfig::default());
        assert_eq!(app.ui.mode, Mode::Search);
        assert!(app.show_preview());
        assert_eq!(app.search_session.settings.mode, SearchMode::Path);
    }

    #[test]
    fn grep_mode_sets_content_state() {
        let mut search_config = search_config();
        search_config.settings.mode = SearchMode::Grep;

        let app = App::from_configs(run_config(), search_config, LoadedAppConfig::default());
        assert!(app.is_content_mode());
        assert_eq!(app.search_session.settings.mode, SearchMode::Grep);
    }

    #[test]
    fn stdin_mode_without_preview_command_hides_preview() {
        let mut run_config = run_config();
        run_config.stdin = true;

        let app = App::from_configs(run_config, search_config(), LoadedAppConfig::default());
        assert!(!app.show_preview());
    }

    #[test]
    fn log_mode_starts_in_preview() {
        let mut run_config = run_config();
        run_config.log = true;

        let app = App::from_configs(run_config, search_config(), LoadedAppConfig::default());
        assert_eq!(app.ui.mode, Mode::Preview);
    }

    #[test]
    fn direct_diff_mode_starts_in_preview() {
        let mut run_config = run_config();
        run_config.diff = Some(["left.txt".into(), "right.txt".into()]);

        let app = App::from_configs(run_config, search_config(), LoadedAppConfig::default());
        assert_eq!(app.ui.mode, Mode::Preview);
        assert!(app.show_preview());
    }
}
