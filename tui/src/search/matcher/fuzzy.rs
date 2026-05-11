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

            let end = (item_limit as u64).min(total_matches);
            let matched_items = snapshot.matched_items(0..end as u32);

            let results: Vec<SearchResult> = matched_items
                .map(|item| {
                    let mut indices = Vec::new();
                    let match_text = item.data.match_text(use_filename_only);
                    let utf32_text = Utf32String::from(match_text.as_ref());

                    let pattern = nucleo.pattern.column_pattern(0);
                    let _ =
                        pattern.indices(utf32_text.slice(..), &mut indices_matcher, &mut indices);

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

            if items_processed > 0 {
                last_items_received = std::time::Instant::now();
            } else if !search_complete
                && !idle_timed_out
                && last_items_received.elapsed() > Duration::from_millis(100)
            {
                idle_timed_out = true;
            }

            let working = (status.running || !search_complete) && !idle_timed_out;
            let should_send = items_processed > 0
                || needs_reparse
                || resized
                || last_sent_total_matches != Some(total_matches)
                || last_sent_working != Some(working);

            if should_send {
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
