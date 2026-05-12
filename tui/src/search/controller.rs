use crate::app::{App, AppEvent, Search};
use crate::infra::channel::{self, MapSender, Sender};
use crate::search::matcher::{spawn_exact_matcher, spawn_matcher, MatcherCommand};
use crate::search::sources::{
    spawn_git_searcher, spawn_searcher_with_config, spawn_stdin_searcher,
};
use crate::search::types::{SearchConfig, SearchItem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

enum SearchInputSource {
    Filesystem,
    GitSearch,
    Stdin(Arc<[String]>),
}

impl SearchInputSource {
    fn spawn_items(&self) -> Option<Vec<String>> {
        match self {
            Self::Filesystem => None,
            Self::GitSearch => None,
            Self::Stdin(items) => Some(items.iter().cloned().collect()),
        }
    }
}

struct ActiveSearchRun {
    epoch: u64,
    stop: Arc<AtomicBool>,
    tx_cmd: channel::DefaultSender<MatcherCommand>,
    _searcher_handle: std::thread::JoinHandle<()>,
    _matcher_handle: std::thread::JoinHandle<()>,
}

impl ActiveSearchRun {
    fn spawn(
        search_config: SearchConfig,
        stdin_items: Option<Vec<String>>,
        tx_main: &channel::DefaultSender<AppEvent>,
        epoch: u64,
    ) -> Self {
        let settings = search_config.settings;
        let stop = Arc::new(AtomicBool::new(false));
        let (tx_items, rx_items) = channel::unbounded_default::<Vec<SearchItem>>();
        let (tx_cmd, rx_cmd) = channel::unbounded_default::<MatcherCommand>();
        let tx_state = MapSender::new(tx_main.clone(), move |state| {
            AppEvent::Matcher(state, epoch)
        });

        let searcher_handle = if let Some(scope) = search_config.git_search_scope.clone() {
            spawn_git_searcher(scope, stop.clone(), tx_items)
        } else if let Some(items) = stdin_items {
            spawn_stdin_searcher(items, stop.clone(), tx_items)
        } else {
            spawn_searcher_with_config(search_config, stop.clone(), tx_items)
        };

        let matcher_handle = if settings.matcher.is_exact() {
            spawn_exact_matcher(
                rx_items,
                rx_cmd,
                stop.clone(),
                tx_state,
                settings.mode.is_file_name_only(),
                settings.mode.is_content(),
                String::new(),
            )
        } else {
            spawn_matcher(
                rx_items,
                rx_cmd,
                stop.clone(),
                tx_state,
                settings.mode.is_file_name_only(),
                settings.mode.is_content(),
            )
        };

        Self {
            epoch,
            stop,
            tx_cmd,
            _searcher_handle: searcher_handle,
            _matcher_handle: matcher_handle,
        }
    }

    fn epoch(&self) -> u64 {
        self.epoch
    }

    fn command_sender(&self) -> &channel::DefaultSender<MatcherCommand> {
        &self.tx_cmd
    }

    fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub struct SearchController {
    tx_main: channel::DefaultSender<AppEvent>,
    source: SearchInputSource,
    active: Option<ActiveSearchRun>,
    next_epoch: u64,
}

impl SearchController {
    pub fn from_search_config(
        search_config: SearchConfig,
        stdin_items: Option<Vec<String>>,
        tx_main: &channel::DefaultSender<AppEvent>,
        search_enabled: bool,
    ) -> Self {
        let source = if search_config.git_search_scope.is_some() {
            SearchInputSource::GitSearch
        } else {
            stdin_items.map_or(SearchInputSource::Filesystem, |items| {
                SearchInputSource::Stdin(Arc::<[String]>::from(items))
            })
        };
        let mut manager = Self {
            tx_main: tx_main.clone(),
            source,
            active: None,
            next_epoch: 0,
        };
        if search_enabled {
            manager.restart(search_config);
        }
        manager
    }

    pub fn command_sender(&self) -> Option<&channel::DefaultSender<MatcherCommand>> {
        self.active.as_ref().map(ActiveSearchRun::command_sender)
    }

    pub fn send_query(&self, query: &str) {
        if query.is_empty() {
            return;
        }
        if let Some(tx_cmd) = self.command_sender() {
            let _ = tx_cmd.send(MatcherCommand::Query(query.to_owned()));
        }
    }

    pub fn accepts_epoch(&self, epoch: u64) -> bool {
        self.active
            .as_ref()
            .map(ActiveSearchRun::epoch)
            .is_some_and(|active_epoch| active_epoch == epoch)
    }

    pub fn reconcile(&mut self, app: &mut App, item_limit: &mut u32) {
        if !app.ui.restart_search {
            return;
        }

        app.ui.restart_search = false;
        self.restart(app.search_config());
        *item_limit = 100;
        app.search_session.search = Search::default();
        app.preview_session.preview.source = None;
        app.preview_session.preview.content = None;
        self.send_query(&app.search_session.query.text);
    }

    pub fn shutdown(&mut self) {
        if let Some(session) = self.active.take() {
            session.shutdown();
        }
    }

    fn restart(&mut self, search_config: SearchConfig) {
        if let Some(session) = self.active.take() {
            session.shutdown();
        }

        let epoch = self.next_epoch;
        self.next_epoch += 1;
        self.active = Some(ActiveSearchRun::spawn(
            search_config,
            self.source.spawn_items(),
            &self.tx_main,
            epoch,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoadedAppConfig;
    use crate::runtime::config::RunConfig;
    use crate::search::types::{MatcherMode, SearchMode, SearchSettings};

    fn run_config() -> RunConfig {
        RunConfig {
            headless: false,
            output_format: crate::cli::args::OutputFormat::Plain,
            output_file: None,
            stdin: true,
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
            query: Some("alpha".to_string()),
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

    fn app() -> App {
        App::from_configs(run_config(), search_config(), LoadedAppConfig::default())
    }

    #[test]
    fn repeated_search_mode_toggles_restart_sessions() {
        let (tx_main, _rx_main) = channel::unbounded_default::<AppEvent>();
        let mut manager = SearchController::from_search_config(
            search_config(),
            Some(vec!["alpha".to_string(), "beta".to_string()]),
            &tx_main,
            true,
        );
        let mut app = app();
        let mut item_limit = 250;

        assert!(manager.accepts_epoch(0));

        app.apply_action(crate::app::AppAction::SetSearchMode(SearchMode::Files));
        manager.reconcile(&mut app, &mut item_limit);
        assert_eq!(app.search_session.settings.mode, SearchMode::Files);
        assert_eq!(item_limit, 100);
        assert!(manager.accepts_epoch(1));

        app.apply_action(crate::app::AppAction::SetSearchMode(SearchMode::Grep));
        manager.reconcile(&mut app, &mut item_limit);
        assert_eq!(app.search_session.settings.mode, SearchMode::Grep);
        assert!(manager.accepts_epoch(2));

        manager.shutdown();
    }

    #[test]
    fn exact_toggle_restarts_with_new_matcher_mode() {
        let (tx_main, _rx_main) = channel::unbounded_default::<AppEvent>();
        let mut manager = SearchController::from_search_config(
            search_config(),
            Some(vec!["alpha".to_string()]),
            &tx_main,
            true,
        );
        let mut app = app();
        let mut item_limit = 100;

        assert_eq!(app.search_session.settings.matcher, MatcherMode::Fuzzy);
        app.apply_action(crate::app::AppAction::ToggleExactMatcher);
        manager.reconcile(&mut app, &mut item_limit);
        assert_eq!(app.search_session.settings.matcher, MatcherMode::Exact);
        assert!(manager.accepts_epoch(1));

        app.apply_action(crate::app::AppAction::ToggleExactMatcher);
        manager.reconcile(&mut app, &mut item_limit);
        assert_eq!(app.search_session.settings.matcher, MatcherMode::Fuzzy);
        assert!(manager.accepts_epoch(2));

        manager.shutdown();
    }
}
