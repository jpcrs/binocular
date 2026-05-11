use crate::infra::channel::{Receiver, Sender};
use crate::search::types::{SearchItem, SearchResult};
use memchr::memmem::Finder;
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::{MatcherCommand, MatcherState};

const MAX_ITEMS_PER_TICK: usize = 100_000;

/// Number of items filtered per chunk during a re-filter pass.
///
/// After each chunk the command queue is drained, so a superseding query can
/// abort and restart the filter within this many items' worth of work (~5–15 ms).
const CHUNK_SIZE: usize = 100_000;

struct QueryFinders {
    finders: Vec<Finder<'static>>,
    tokens: Vec<String>,
    /// Pre-computed char lengths for each token (avoids O(m) recount per match).
    token_char_lens: Vec<usize>,
}

impl QueryFinders {
    fn new(query: &str) -> Self {
        let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
        let finders = tokens
            .iter()
            .map(|t| Finder::new(t.as_bytes()).into_owned())
            .collect();
        let token_char_lens = tokens.iter().map(|t| t.chars().count()).collect();
        Self {
            finders,
            tokens,
            token_char_lens,
        }
    }

    fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Returns true if all tokens appear in `haystack` (already lowercased).
    fn matches(&self, haystack: &[u8]) -> bool {
        self.finders.iter().all(|f| f.find(haystack).is_some())
    }
}

fn compute_indices(item: &str, tokens: &[String], token_char_lens: &[usize]) -> Vec<u32> {
    let lower = item.to_lowercase();
    let lower_bytes = lower.as_bytes();
    let is_ascii = lower.is_ascii();
    let mut indices = Vec::new();

    for (token, &token_char_len) in tokens.iter().zip(token_char_lens.iter()) {
        if token.is_empty() {
            continue;
        }
        let finder = Finder::new(token.as_bytes());
        let mut start = 0usize;

        while let Some(rel) = finder.find(&lower_bytes[start..]) {
            let abs = start + rel;
            let char_start = if is_ascii {
                abs
            } else {
                lower[..abs].chars().count()
            };
            for i in char_start..char_start + token_char_len {
                indices.push(i as u32);
            }
            start = abs + token.len().max(1);
        }
    }

    indices.sort_unstable();
    indices.dedup();
    indices
}

fn build_results(
    items: &[SearchItem],
    match_indices: &[usize],
    finders: &QueryFinders,
    is_content: bool,
    limit: usize,
) -> Vec<SearchResult> {
    let visible_end = limit.min(match_indices.len());
    match_indices[..visible_end]
        .iter()
        .map(|&idx| {
            let item = &items[idx];
            let indices = if finders.is_empty() {
                vec![]
            } else {
                compute_indices(
                    item.display_text().as_ref(),
                    &finders.tokens,
                    &finders.token_char_lens,
                )
            };
            let col = if is_content {
                item.content_match_column(&indices)
            } else {
                None
            };
            SearchResult {
                item: item.clone(),
                indices,
                column: col,
            }
        })
        .collect()
}

pub fn spawn_exact_matcher(
    rx_items: impl Receiver<Vec<SearchItem>> + Send + 'static,
    rx_cmd: impl Receiver<MatcherCommand> + Send + 'static,
    stop: Arc<AtomicBool>,
    tx_state: impl Sender<MatcherState> + Send + 'static,
    use_filename_only: bool,
    is_content: bool,
    initial_query: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // Dedicated thread pool: leave at least 2 cores for the UI and input
        // threads so Rayon cannot fully starve them during a parallel re-filter.
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(2).max(1))
            .unwrap_or(2);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap_or_else(|_| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(1)
                    .build()
                    .expect("rayon thread pool")
            });

        run(
            rx_items,
            rx_cmd,
            stop,
            tx_state,
            use_filename_only,
            is_content,
            &pool,
            initial_query,
        );
    })
}

// Main loop
fn run(
    rx_items: impl Receiver<Vec<SearchItem>>,
    rx_cmd: impl Receiver<MatcherCommand>,
    stop: Arc<AtomicBool>,
    tx_state: impl Sender<MatcherState>,
    use_filename_only: bool,
    is_content: bool,
    pool: &rayon::ThreadPool,
    initial_query: String,
) {
    let mut items: Vec<SearchItem> = Vec::new();
    let mut match_lower: Vec<Vec<u8>> = Vec::new();

    let mut all_matches: Vec<usize> = Vec::new();
    let mut total_matches: u64 = 0;

    let mut finders = QueryFinders::new(&initial_query);
    let mut current_query = initial_query;
    let mut item_limit: u32 = 100;
    let mut search_complete = false;
    let mut last_sent_total_matches: Option<u64> = None;
    let mut last_sent_working: Option<bool> = None;
    let mut last_items_received = std::time::Instant::now();
    let mut idle_timed_out = false;

    while !stop.load(Ordering::Relaxed) {
        // Ingest new items from the searcher
        let mut items_processed = 0usize;

        if !search_complete {
            loop {
                match rx_items.try_recv() {
                    Ok(Some(batch)) => {
                        let new_start = items.len();

                        for item in batch {
                            let filterable = item.match_text(use_filename_only).to_lowercase();
                            match_lower.push(filterable.into_bytes());
                            items.push(item);
                        }

                        let new_count = items.len() - new_start;
                        items_processed += new_count;

                        // Incrementally test new items against current finders.
                        for idx in new_start..items.len() {
                            if finders.is_empty() || finders.matches(&match_lower[idx]) {
                                all_matches.push(idx);
                            }
                        }

                        if items_processed >= MAX_ITEMS_PER_TICK {
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

        // Process commands
        let mut needs_refilter = false;
        let mut resized = false;

        while let Ok(Some(cmd)) = rx_cmd.try_recv() {
            match cmd {
                MatcherCommand::Query(q) => {
                    if q != current_query {
                        current_query = q.clone();
                        finders = QueryFinders::new(&q);
                        needs_refilter = true;
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

        if needs_refilter {
            'refilter: loop {
                let snapshot_len = items.len();
                all_matches.clear();

                if finders.is_empty() {
                    // Empty query: all items match
                    all_matches = (0..snapshot_len).collect();
                } else {
                    let mut chunk_start = 0usize;
                    while chunk_start < snapshot_len {
                        let chunk_end = (chunk_start + CHUNK_SIZE).min(snapshot_len);

                        // Parallel filter for this chunk using the limited pool
                        let chunk_result: Vec<usize> = pool.install(|| {
                            (chunk_start..chunk_end)
                                .into_par_iter()
                                .filter(|&idx| finders.matches(&match_lower[idx]))
                                .collect()
                        });
                        all_matches.extend(chunk_result);
                        chunk_start = chunk_end;

                        // Drain commands between chunks
                        let mut query_changed = false;
                        while let Ok(Some(cmd)) = rx_cmd.try_recv() {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            match cmd {
                                MatcherCommand::Query(q) => {
                                    if q != current_query {
                                        current_query = q.clone();
                                        finders = QueryFinders::new(&q);
                                        query_changed = true;
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

                        // Send partial results so the UI stays alive.
                        // Always mark working=true here: we are actively filtering,
                        // regardless of whether the searcher has already finished.
                        let working = true;
                        total_matches = all_matches.len() as u64;
                        let results = build_results(
                            &items,
                            &all_matches,
                            &finders,
                            is_content,
                            item_limit as usize,
                        );
                        let _ = tx_state.try_send(MatcherState {
                            results,
                            total_matches,
                            total_items: items.len() as u64,
                            working,
                        });
                        last_sent_total_matches = Some(total_matches);
                        last_sent_working = Some(working);

                        if query_changed {
                            // Restart the entire filter with the new finders.
                            continue 'refilter;
                        }
                    }
                }

                // Re-filter completed without interruption.
                break 'refilter;
            }

            total_matches = all_matches.len() as u64;
        }

        // Update total match count
        if items_processed > 0 {
            total_matches = all_matches.len() as u64;
        }

        // Idle timeout
        if items_processed > 0 {
            last_items_received = std::time::Instant::now();
        } else if !search_complete
            && !idle_timed_out
            && last_items_received.elapsed() > Duration::from_millis(100)
        {
            idle_timed_out = true;
        }

        // Send state if anything changed
        let working = !search_complete && !idle_timed_out;
        let should_send = items_processed > 0
            || resized
            || last_sent_total_matches != Some(total_matches)
            || last_sent_working != Some(working);

        if should_send {
            let results = build_results(
                &items,
                &all_matches,
                &finders,
                is_content,
                item_limit as usize,
            );
            let _ = tx_state.try_send(MatcherState {
                results,
                total_matches,
                total_items: items.len() as u64,
                working,
            });
            last_sent_total_matches = Some(total_matches);
            last_sent_working = Some(working);
        }

        if !should_send {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
