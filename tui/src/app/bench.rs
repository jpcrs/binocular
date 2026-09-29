use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const SAMPLE_LIMIT: usize = 120;
pub const DRAW_BUDGET: Duration = Duration::from_micros(16_667);

/// Bounded samples keep instrumentation cost and memory independent of session length.
#[derive(Default)]
pub struct Timings {
    samples: VecDeque<Duration>,
    pub count: u64,
}

pub struct TimingSummary {
    pub last: Duration,
    pub mean: Duration,
    pub p95: Duration,
    pub max: Duration,
    pub over_budget: usize,
    pub samples: usize,
}

impl Timings {
    pub fn record(&mut self, elapsed: Duration) {
        if self.samples.len() == SAMPLE_LIMIT {
            self.samples.pop_front();
        }
        self.samples.push_back(elapsed);
        self.count += 1;
    }

    pub fn summary(&self) -> Option<TimingSummary> {
        let last = *self.samples.back()?;
        let mut sorted: Vec<_> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        Some(TimingSummary {
            last,
            mean: sorted.iter().sum::<Duration>() / sorted.len() as u32,
            p95: sorted[(sorted.len() * 95).div_ceil(100) - 1],
            max: *sorted.last().unwrap(),
            over_budget: sorted.iter().filter(|&&d| d > DRAW_BUDGET).count(),
            samples: sorted.len(),
        })
    }
}

pub struct BenchState {
    pub visible: bool,
    pub scroll: u16,
    pub started_at: Instant,
    pub first_frame: Option<Duration>,
    pub first_results: Option<Duration>,
    pub first_preview: Option<Duration>,
    pub draws: Timings,
    pub event_batches: Timings,
}

impl BenchState {
    pub fn new(started_at: Instant) -> Self {
        Self {
            visible: true,
            scroll: 0,
            started_at,
            first_frame: None,
            first_results: None,
            first_preview: None,
            draws: Timings::default(),
            event_batches: Timings::default(),
        }
    }

    pub fn record_draw(&mut self, elapsed: Duration) {
        self.first_frame
            .get_or_insert_with(|| self.started_at.elapsed());
        self.draws.record(elapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_use_nearest_rank_and_evict_old_samples() {
        let mut timings = Timings::default();
        assert!(timings.summary().is_none());
        timings.record(Duration::from_secs(10));
        for ms in 1..=120 {
            timings.record(Duration::from_millis(ms));
        }
        let summary = timings.summary().unwrap();
        assert_eq!(timings.count, 121);
        assert_eq!(summary.samples, SAMPLE_LIMIT);
        assert_eq!(summary.last, Duration::from_millis(120));
        assert_eq!(summary.mean, Duration::from_micros(60_500));
        assert_eq!(summary.p95, Duration::from_millis(114));
        assert_eq!(summary.max, Duration::from_millis(120));
        assert_eq!(summary.over_budget, 104);
    }

    #[test]
    fn first_frame_is_preserved_across_draws() {
        let mut bench = BenchState::new(Instant::now() - Duration::from_secs(1));
        bench.record_draw(Duration::from_millis(2));
        let first = bench.first_frame;
        bench.record_draw(Duration::from_millis(3));
        assert!(first.unwrap() >= Duration::from_secs(1));
        assert_eq!(bench.first_frame, first);
        assert_eq!(bench.draws.count, 2);
    }
}
