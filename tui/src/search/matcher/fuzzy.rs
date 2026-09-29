use crate::infra::channel::{ChannelError, Receiver, Sender};
use crate::search::types::{SearchItem, SearchResult};
use nucleo::{Config, Matcher, Nucleo, Utf32String};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_ITEMS_PER_TICK: usize = 4096;
const INGEST_BUDGET: Duration = Duration::from_millis(2);
const IDLE_WAIT: Duration = Duration::from_millis(10);

pub struct MatcherState {
    /// Keep this many previously published rows, then replace the remaining tail.
    /// States must be applied in order, including status-only (empty tail) updates.
    pub results_start: usize,
    pub results: Vec<SearchResult>,
    pub total_matches: u64,
    pub total_items: u64,
    pub working: bool,
}

impl MatcherState {
    pub fn apply_results(&mut self, rows: &mut Vec<SearchResult>) {
        assert!(self.results_start <= rows.len(), "missing matcher update");
        rows.truncate(self.results_start);
        rows.append(&mut self.results);
    }
}

pub enum MatcherCommand {
    Query(String),
    Resize(u32),
}

// Operators and escapes can broaden a pattern when extended (e.g. !foo -> !foox).
// Restrict incremental matching to plain extensions whose matches can only narrow.
fn can_append(previous: &str, next: &str) -> bool {
    next.len() > previous.len()
        && next.starts_with(previous)
        && !next.contains(['!', '^', '$', '\'', '\\'])
}

pub fn spawn_matcher(
    rx_items: impl Receiver<Vec<SearchItem>>,
    rx_cmd: impl Receiver<MatcherCommand>,
    stop: Arc<AtomicBool>,
    tx_state: impl Sender<MatcherState>,
    use_filename_only: bool,
    is_content: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut nucleo = Nucleo::<SearchItem>::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
        let injector = nucleo.injector();
        let mut current_query = String::new();
        let mut parsed_query = String::new();
        let mut item_limit = 100;
        let mut search_complete = false;
        let mut pending_items = Vec::new().into_iter();
        let mut pending_command = None;
        let mut commands_closed = false;
        let mut last_metadata = None;
        let mut published_end = 0;
        let mut replace_results = true;
        let mut dirty = true;
        let mut indices_matcher = Matcher::new(Config::DEFAULT);

        while !stop.load(Ordering::Relaxed) {
            // Drain commands before ingestion, including the command that woke us.
            loop {
                let command = pending_command.take().or_else(|| match rx_cmd.try_recv() {
                    Ok(command) => command,
                    Err(_) => {
                        commands_closed = true;
                        None
                    }
                });
                match command {
                    Some(MatcherCommand::Query(query)) => current_query = query,
                    Some(MatcherCommand::Resize(limit)) => {
                        item_limit = limit;
                        dirty = true; // Acknowledge even a resize with no new matches.
                    }
                    None => break,
                }
            }
            if current_query != parsed_query {
                nucleo.pattern.reparse(
                    0,
                    &current_query,
                    nucleo::pattern::CaseMatching::Smart,
                    nucleo::pattern::Normalization::Smart,
                    can_append(&parsed_query, &current_query),
                );
                parsed_query.clone_from(&current_query);
                dirty = true;
            }

            // Retain the unconsumed tail even when a producer sends a huge batch.
            let started = Instant::now();
            let mut items_processed = 0;
            while !search_complete && items_processed < MAX_ITEMS_PER_TICK {
                if items_processed % 64 == 0
                    && (stop.load(Ordering::Relaxed) || started.elapsed() >= INGEST_BUDGET)
                {
                    break;
                }
                if let Some(item) = pending_items.next() {
                    injector.push(item, |item_ref, cols: &mut [Utf32String]| {
                        cols[0] =
                            Utf32String::from(item_ref.match_text(use_filename_only).as_ref());
                    });
                    items_processed += 1;
                } else {
                    match rx_items.try_recv() {
                        Ok(Some(batch)) => pending_items = batch.into_iter(),
                        Ok(None) => break,
                        Err(_) => search_complete = true,
                    }
                }
            }

            let status = nucleo.tick(1);
            let snapshot = nucleo.snapshot();
            let total_matches = snapshot.matched_item_count() as u64;
            let total_items = snapshot.item_count() as u64;
            // A quiet source is still loading until it closes; never mask a running match.
            let working = status.running || !search_complete;
            let metadata = (total_matches, total_items, working);
            replace_results |= status.changed;
            dirty |= replace_results || last_metadata != Some(metadata);

            if dirty {
                let end = (item_limit as u64).min(total_matches) as usize;
                let start = if replace_results {
                    0
                } else {
                    published_end.min(end)
                };
                let results = snapshot
                    .matched_items(start as u32..end as u32)
                    .map(|item| {
                        let mut indices = Vec::new();
                        let _ = snapshot.pattern().column_pattern(0).indices(
                            item.matcher_columns[0].slice(..),
                            &mut indices_matcher,
                            &mut indices,
                        );
                        let column = if is_content {
                            item.data.content_match_column(&indices)
                        } else {
                            None
                        };
                        SearchResult {
                            item: item.data.clone(),
                            indices,
                            column,
                        }
                    })
                    .collect();
                match tx_state.try_send(MatcherState {
                    results_start: start,
                    results,
                    total_matches,
                    total_items,
                    working,
                }) {
                    Ok(()) => {
                        published_end = end;
                        last_metadata = Some(metadata);
                        replace_results = false;
                        dirty = false;
                    }
                    Err(ChannelError::Disconnected) => break,
                    Err(_) => {} // Retry against the last successfully published prefix.
                }
            }

            if !status.running && items_processed == 0 {
                if commands_closed {
                    if search_complete && !dirty {
                        break;
                    }
                    std::thread::sleep(IDLE_WAIT);
                } else {
                    match rx_cmd.recv_timeout(IDLE_WAIT) {
                        Ok(command) => pending_command = command,
                        Err(_) => commands_closed = true,
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::channel::{self, DefaultReceiver, DefaultSender};
    use std::time::Instant;

    struct Worker {
        commands: DefaultSender<MatcherCommand>,
        states: DefaultReceiver<MatcherState>,
        stop: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
        rows: std::cell::RefCell<Vec<SearchResult>>,
    }

    impl Worker {
        fn new(filename: bool, content: bool) -> (Self, DefaultSender<Vec<SearchItem>>) {
            let (items, rx_items) = channel::unbounded_default();
            let (commands, rx_cmd) = channel::unbounded_default();
            let (tx_state, states) = channel::unbounded_default();
            let stop = Arc::new(AtomicBool::new(false));
            let handle = Some(spawn_matcher(
                rx_items,
                rx_cmd,
                stop.clone(),
                tx_state,
                filename,
                content,
            ));
            (
                Self {
                    commands,
                    states,
                    stop,
                    handle,
                    rows: Default::default(),
                },
                items,
            )
        }

        fn until(&self, predicate: impl Fn(&MatcherState) -> bool) -> MatcherState {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Some(mut state) = self.states.try_recv().unwrap() {
                    state.apply_results(&mut self.rows.borrow_mut());
                    state.results = self.rows.borrow().clone();
                    state.results_start = 0;
                    if predicate(&state) {
                        return state;
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            panic!("matcher failed to publish the expected state");
        }
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            self.handle.take().unwrap().join().unwrap();
        }
    }

    #[test]
    fn queries_with_equal_counts_resize_and_streaming_still_publish_results() {
        let (worker, items) = Worker::new(false, false);
        worker
            .commands
            .send(MatcherCommand::Query("alpha".into()))
            .unwrap();
        items
            .send(
                (0..300)
                    .map(|i| SearchItem::path(format!("alpha_{i:03}")))
                    .collect(),
            )
            .unwrap();
        worker.until(|s| s.total_matches == 300 && s.results.len() == 100);
        items
            .send(
                (0..300)
                    .map(|i| SearchItem::path(format!("beta_{i:03}")))
                    .collect(),
            )
            .unwrap();
        drop(items);
        worker.until(|s| !s.working && s.total_items == 600);
        worker
            .commands
            .send(MatcherCommand::Query("beta".into()))
            .unwrap();
        let state = worker.until(|s| {
            !s.working
                && s.total_matches == 300
                && s.results
                    .iter()
                    .all(|r| r.item.match_text(false).starts_with("beta"))
        });
        assert!(state.results.iter().all(|r| r.indices == [0, 1, 2, 3]));
        worker.commands.send(MatcherCommand::Resize(250)).unwrap();
        worker.until(|s| s.results.len() == 250);
        worker
            .commands
            .send(MatcherCommand::Query("no-match".into()))
            .unwrap();
        worker.until(|s| !s.working && s.total_matches == 0 && s.results.is_empty());
        worker
            .commands
            .send(MatcherCommand::Query(String::new()))
            .unwrap();
        let state = worker.until(|s| !s.working && s.total_matches == 600);
        assert_eq!(state.results.len(), 250);
        assert!(state.results.iter().all(|r| r.indices.is_empty()));
    }

    #[test]
    fn cached_match_columns_preserve_filename_and_unicode_content_highlights() {
        for filename in [false, true] {
            let (worker, items) = Worker::new(filename, !filename);
            let item = if filename {
                SearchItem::path("dir/café.rs")
            } else {
                SearchItem::grep("src/main.rs", 4, "let café = 1;")
            };
            worker
                .commands
                .send(MatcherCommand::Query("café".into()))
                .unwrap();
            items.send(vec![item.clone()]).unwrap();
            drop(items);
            let state = worker.until(|s| !s.working && s.total_matches == 1);
            let result = &state.results[0];
            assert_eq!(result.item, item);
            let matched: String = item
                .match_text(filename)
                .chars()
                .enumerate()
                .filter(|(i, _)| result.indices.contains(&(*i as u32)))
                .map(|(_, c)| c)
                .collect();
            assert_eq!(matched, "café");
            assert_eq!(result.column, if filename { None } else { Some(5) });
        }
    }
    #[test]
    fn incremental_queries_match_full_rescoring() {
        use nucleo::pattern::{CaseMatching, Normalization};
        let corpus: Vec<_> = [
            "src/service.rs",
            "src/Service.rs",
            "src/server.rs",
            "service 000",
            "service 001",
            "foo",
            "foox",
            "foo$x",
            "xfoo",
            "!foo",
            "foo bar",
            "foo\\bar",
            "foo'bar",
            "cafe",
            "café",
            "cafÉ",
            "cafe\u{301}",
            "cafe\u{301}x",
            "日本語",
            "日本語.rs",
            "🦀rust",
        ]
        .into_iter()
        .map(SearchItem::path)
        .collect();
        let (worker, items) = Worker::new(false, false);
        worker.commands.send(MatcherCommand::Resize(1000)).unwrap();
        items.send(corpus.clone()).unwrap();
        drop(items);
        worker.until(|s| !s.working && s.total_items == corpus.len() as u64);
        let mut oracle = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
        for item in corpus {
            oracle.injector().push(item, |item, cols| {
                cols[0] = Utf32String::from(item.match_text(false).as_ref());
            });
        }
        let queries = [
            "s",
            "se",
            "ser",
            "serv",
            "service",
            "service ",
            "service 0",
            "service 00",
            "service 000",
            "service 00",
            "Service",
            "S",
            "foo",
            "foo$",
            "foo$x",
            "!foo",
            "!foox",
            "^foo",
            "^foox",
            "'foo",
            "'foox",
            "foo\\",
            "foo\\b",
            "foo'",
            "foo'b",
            "c",
            "ca",
            "caf",
            "cafe",
            "cafe\u{301}",
            "cafe\u{301}x",
            "café",
            "cafÉ",
            "日",
            "日本",
            "日本語",
            "🦀",
            "🦀r",
            "",
        ];
        for query in queries {
            worker
                .commands
                .send(MatcherCommand::Query(query.into()))
                .unwrap();
            let state = worker.until(|s| !s.working);
            oracle
                .pattern
                .reparse(0, query, CaseMatching::Smart, Normalization::Smart, false);
            let deadline = Instant::now() + Duration::from_secs(5);
            while oracle.tick(10).running {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            let snapshot = oracle.snapshot();
            let mut matcher = Matcher::new(Config::DEFAULT);
            let expected: Vec<_> = snapshot
                .matched_items(..)
                .map(|item| {
                    let mut indices = Vec::new();
                    snapshot.pattern().column_pattern(0).indices(
                        item.matcher_columns[0].slice(..),
                        &mut matcher,
                        &mut indices,
                    );
                    (item.data.clone(), indices)
                })
                .collect();
            let actual: Vec<_> = state
                .results
                .into_iter()
                .map(|r| (r.item, r.indices))
                .collect();
            assert_eq!(actual, expected, "query {query:?}");
        }
        assert!(can_append("serv", "service"));
        assert!(!can_append("foo$", "foo$x"));
        assert!(!can_append("!foo", "!foox"));
        assert!(!can_append("service", "serv"));
    }

    #[test]
    fn pagination_sends_only_the_tail_and_shrink_preserves_prefix() {
        let (worker, items) = Worker::new(false, false);
        items
            .send(
                (0..300)
                    .map(|i| SearchItem::path(format!("item{i:03}")))
                    .collect(),
            )
            .unwrap();
        drop(items);
        let initial = worker.until(|s| !s.working && s.total_items == 300);
        let mut rows = initial.results;
        for (limit, start, count) in [(250, 100, 150), (250, 250, 0), (40, 40, 0), (100, 40, 60)] {
            worker.commands.send(MatcherCommand::Resize(limit)).unwrap();
            let mut state = worker
                .states
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!((state.results_start, state.results.len()), (start, count));
            state.apply_results(&mut rows);
            assert_eq!(rows.len(), limit as usize);
            assert!(rows
                .windows(2)
                .all(|w| w[0].item.match_text(false) < w[1].item.match_text(false)));
        }
    }

    #[test]
    fn giant_batches_publish_progress_and_quiet_sources_remain_working() {
        let (worker, items) = Worker::new(false, false);
        let count = 100_000;
        items
            .send(
                (0..count)
                    .map(|i| SearchItem::path(format!("item{i}")))
                    .collect(),
            )
            .unwrap();
        let partial = worker.until(|s| s.total_items > 0);
        assert!(partial.total_items < count);
        assert!(partial.working);
        worker.until(|s| s.total_items == count);
        std::thread::sleep(Duration::from_millis(130));
        while let Some(state) = worker.states.try_recv().unwrap() {
            assert!(state.working, "an open source must not report completion");
            // These are metadata updates; preserve the test harness cache as well.
            let mut state = state;
            state.apply_results(&mut worker.rows.borrow_mut());
        }
        worker
            .commands
            .send(MatcherCommand::Query("item99999".into()))
            .unwrap();
        worker.until(|s| s.total_matches == 1 && !s.results[0].indices.is_empty());
        items
            .send(vec![SearchItem::path("another_item99999")])
            .unwrap();
        drop(items);
        let state = worker.until(|s| !s.working && s.total_items == count + 1);
        assert_eq!(state.total_matches, 2);
    }

    #[test]
    fn full_state_channel_retries_without_losing_result_prefix() {
        let (tx_items, rx_items) = channel::unbounded_default();
        let (tx_cmd, rx_cmd) = channel::unbounded_default();
        let (tx_state, rx_state) = channel::bounded_default(1);
        let stop = Arc::new(AtomicBool::new(false));
        // Queue commands before starting to exercise coalescing against the parsed query.
        tx_cmd.send(MatcherCommand::Query("none".into())).unwrap();
        tx_cmd.send(MatcherCommand::Query("item".into())).unwrap();
        tx_cmd.send(MatcherCommand::Resize(250)).unwrap();
        tx_items
            .send(
                (0..300)
                    .map(|i| SearchItem::path(format!("item{i}")))
                    .collect(),
            )
            .unwrap();
        drop(tx_items);
        let worker = spawn_matcher(rx_items, rx_cmd, stop.clone(), tx_state, false, false);
        std::thread::sleep(Duration::from_millis(30));
        let mut rows = Vec::new();
        loop {
            let mut state = rx_state
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            state.apply_results(&mut rows);
            if !state.working && state.total_matches == 300 {
                assert_eq!(rows.len(), 250);
                assert!(rows.iter().all(|r| r.indices == [0, 1, 2, 3]));
                break;
            }
        }
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
    }
}
