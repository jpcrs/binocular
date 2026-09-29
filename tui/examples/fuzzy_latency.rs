//! Release-mode query/pagination benchmark. Run with no other build jobs active.
use binocular::infra::channel::{self, Receiver, Sender};
use binocular::search::matcher::{spawn_matcher, MatcherCommand, MatcherState};
use binocular::search::types::SearchItem;
use std::hash::{Hash, Hasher};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn receive(rx: &channel::DefaultReceiver<MatcherState>) -> MatcherState {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(state) = rx.try_recv().unwrap() {
            return state;
        }
        assert!(Instant::now() < until, "matcher stalled");
        std::thread::sleep(Duration::from_micros(50));
    }
}
fn main() {
    let (tx_items, rx_items) = channel::unbounded_default();
    let (tx_cmd, rx_cmd) = channel::unbounded_default();
    let (tx_state, rx_state) = channel::unbounded_default();
    let stop = Arc::new(AtomicBool::new(false));
    let worker = spawn_matcher(rx_items, rx_cmd, stop.clone(), tx_state, false, false);
    let start = Instant::now();
    tx_items
        .send(
            (0..200_000)
                .map(|i| {
                    SearchItem::path(format!(
                        "src/{}/{i:06}/module.rs",
                        if i % 10 == 0 { "service" } else { "component" }
                    ))
                })
                .collect(),
        )
        .unwrap();
    drop(tx_items);
    let mut rows = Vec::new();
    loop {
        let mut state = receive(&rx_state);
        state.apply_results(&mut rows);
        if !state.working && state.total_items == 200_000 {
            break;
        }
    }
    println!("index_ms={:.3}", start.elapsed().as_secs_f64() * 1000.);
    let queries = [
        "s",
        "se",
        "ser",
        "serv",
        "servi",
        "servic",
        "service",
        "service 0",
        "service 00",
        "service 000",
        "service 00",
        "service",
        "component",
        "component 1",
        "component 19",
        "component 199",
        "",
    ];
    let mut times = Vec::new();
    let mut digest = std::collections::hash_map::DefaultHasher::new();
    for _ in 0..3 {
        for (i, query) in queries.iter().enumerate() {
            std::thread::sleep(Duration::from_millis(2 + (i % 7) as u64));
            let start = Instant::now();
            tx_cmd.send(MatcherCommand::Query((*query).into())).unwrap();
            loop {
                let mut state = receive(&rx_state);
                state.apply_results(&mut rows);
                if !state.working {
                    times.push(start.elapsed().as_secs_f64() * 1000.);
                    format!("{:?}", rows).hash(&mut digest);
                    state.total_matches.hash(&mut digest);
                    break;
                }
            }
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "typing_median_ms={:.3} typing_p95_ms={:.3} checksum={}",
        times[times.len() / 2],
        times[(times.len() * 95).div_ceil(100) - 1],
        digest.finish()
    );
    let start = Instant::now();
    let mut published = 0;
    for limit in (200..=10_000).step_by(100) {
        tx_cmd.send(MatcherCommand::Resize(limit)).unwrap();
        loop {
            let mut state = receive(&rx_state);
            published += state.results.len();
            state.apply_results(&mut rows);
            if rows.len() == limit as usize {
                break;
            }
        }
    }
    println!(
        "pagination_ms={:.3} rows_published={published} final_rows={}",
        start.elapsed().as_secs_f64() * 1000.,
        rows.len()
    );
    stop.store(true, Ordering::Relaxed);
    drop(tx_cmd);
    worker.join().unwrap();
}
