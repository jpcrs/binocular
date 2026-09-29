use crate::infra::channel::{Receiver, Sender};
use crate::search::types::{SearchItem, SearchResult};
use nucleo::{Config, Injector, Matcher, Nucleo, Utf32String};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MAX_ITEMS_PER_TICK: usize = 100_000;

fn push_batch_to_nucleo(
    injector: &Injector<SearchItem>,
    batch: Vec<SearchItem>,
    use_filename_only: bool,
) -> usize {
    let count = batch.len();
    for item in batch {
        injector.push(item, |item_ref, cols: &mut [Utf32String]| {
            cols[0] = Utf32String::from(item_ref.match_text(use_filename_only).as_ref())
        });
    }
    count
}

pub struct MatcherState {
    pub results: Vec<SearchResult>,
    pub total_matches: u64,
    pub total_items: u64,
    pub working: bool,
}

pub enum MatcherCommand {
    Query(String),
    Resize(u32),
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
        let mut item_limit = 100;
        let mut search_complete = false;
        let mut last_sent_total_matches: Option<u64> = None;
        let mut last_sent_working: Option<bool> = None;
        let mut last_items_received = std::time::Instant::now();
        let mut idle_timed_out = false;

        let mut indices_matcher = Matcher::new(Config::DEFAULT);

        while !stop.load(Ordering::Relaxed) {
            let mut items_processed = 0;

            if !search_complete {
                loop {
                    match rx_items.try_recv() {
                        Ok(Some(batch)) => {
                            items_processed +=
                                push_batch_to_nucleo(&injector, batch, use_filename_only);
                            if items_processed > MAX_ITEMS_PER_TICK {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            search_complete = true;
                            break;
                        }
                    }
                }
            }

            let mut needs_reparse = false;
            let mut resized = false;
            while let Ok(Some(cmd)) = rx_cmd.try_recv() {
                match cmd {
                    MatcherCommand::Query(q) => {
                        if q != current_query {
                            current_query = q;
                            needs_reparse = true;
                        }
                    }
                    MatcherCommand::Resize(n) => {
                        if n != item_limit {
                            item_limit = n;
                            resized = true;
                        }
                    }
                }
            }

            if needs_reparse {
                nucleo.pattern.reparse(
                    0,
                    &current_query,
                    nucleo::pattern::CaseMatching::Smart,
                    nucleo::pattern::Normalization::Smart,
                    false,
                );
            }

            let status = nucleo.tick(10);

            let snapshot = nucleo.snapshot();
            let total_matches = snapshot.matched_item_count() as u64;
            let total_items = snapshot.item_count() as u64;

            if items_processed > 0 {
                last_items_received = std::time::Instant::now();
            } else if !search_complete
                && !idle_timed_out
                && last_items_received.elapsed() > Duration::from_millis(100)
            {
                idle_timed_out = true;
            }

            let working = (status.running || !search_complete) && !idle_timed_out;
            let should_send = status.changed
                || items_processed > 0
                || needs_reparse
                || resized
                || last_sent_total_matches != Some(total_matches)
                || last_sent_working != Some(working);

            if should_send {
                let end = (item_limit as u64).min(total_matches);
                let matched_items = snapshot.matched_items(0..end as u32);

                let results: Vec<SearchResult> = matched_items
                    .map(|item| {
                        let mut indices = Vec::new();

                        let pattern = snapshot.pattern().column_pattern(0);
                        let _ = pattern.indices(
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

                let _ = tx_state.try_send(MatcherState {
                    results,
                    total_matches,
                    total_items,
                    working,
                });
                last_sent_total_matches = Some(total_matches);
                last_sent_working = Some(working);
            }

            if !status.running && items_processed == 0 {
                std::thread::sleep(Duration::from_millis(10));
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
                },
                items,
            )
        }

        fn until(&self, predicate: impl Fn(&MatcherState) -> bool) -> MatcherState {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Some(state) = self.states.try_recv().unwrap() {
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
}
