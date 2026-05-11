use crate::infra::channel::{BatchSender, Sender};
use crate::search::types::SearchItem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn spawn_stdin_searcher(
    items: Vec<String>,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut batcher = BatchSender::new(tx, 4096);
        for item in items {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            batcher.push(SearchItem::stdin(item));
        }
    })
}
