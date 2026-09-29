use crate::app::bench::{TimingSummary, DRAW_BUDGET, SAMPLE_LIMIT};
use crate::app::App;
use crate::config::format_keybindings;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Duration;

fn millis(duration: Duration) -> String {
    format!("{:.2} ms", duration.as_secs_f64() * 1000.0)
}

fn milestone(duration: Option<Duration>, applicable: bool) -> String {
    match (duration, applicable) {
        (Some(duration), _) => millis(duration),
        (None, true) => "Waiting…".into(),
        (None, false) => "N/A".into(),
    }
}

fn timing_lines(label: &str, summary: Option<&TimingSummary>) -> Vec<Line<'static>> {
    let Some(s) = summary else {
        return vec![Line::from(format!("{label}: collecting samples…"))];
    };
    vec![
        Line::from(format!("{label} ({} samples)", s.samples))
            .style(Style::default().fg(Color::LightCyan)),
        Line::from(format!(
            "  Last {:>12}   Mean {:>12}",
            millis(s.last),
            millis(s.mean)
        )),
        Line::from(format!(
            "  p95  {:>12}   Max  {:>12}",
            millis(s.p95),
            millis(s.max)
        )),
    ]
}

pub fn render_bench_modal(f: &mut Frame, app: &mut App) {
    let Some(bench) = app.ui.bench.as_ref().filter(|bench| bench.visible) else {
        return;
    };
    let draws = bench.draws.summary();
    let (status, color) = match draws.as_ref().map(|s| s.p95) {
        None => ("Collecting samples", Color::Gray),
        Some(p95) if p95 <= DRAW_BUDGET => ("Healthy", Color::Green),
        Some(p95) if p95 <= Duration::from_millis(50) => ("Elevated draw time", Color::Yellow),
        Some(_) => ("Slow drawing", Color::Red),
    };
    let search_applicable = !app.runtime.run.log && app.runtime.run.diff.is_none();
    let mut lines = vec![
        Line::from(format!("UI status: {status}")).style(Style::default().fg(color)),
        Line::from("Based on draw p95: healthy ≤16.67 ms, slow >50 ms."),
        Line::from(""),
        Line::from("Load timings (since launch)").style(Style::default().fg(Color::LightCyan)),
        Line::from(format!(
            "  First frame    {}",
            milestone(bench.first_frame, true)
        )),
        Line::from(format!(
            "  First results  {}",
            milestone(bench.first_results, search_applicable)
        )),
        Line::from(format!(
            "  First preview  {}",
            milestone(
                bench.first_preview,
                app.show_preview() || app.runtime.run.log
            )
        )),
        Line::from(format!(
            "  Session age    {:.1} s",
            bench.started_at.elapsed().as_secs_f64()
        )),
        Line::from(""),
    ];
    lines.extend(timing_lines("UI draw + terminal flush", draws.as_ref()));
    if let Some(s) = &draws {
        lines.push(Line::from(format!(
            "  Over 16.67 ms: {}/{} draws",
            s.over_budget, s.samples
        )));
    }
    lines.push(Line::from(""));
    lines.extend(timing_lines(
        "Event batch processing",
        bench.event_batches.summary().as_ref(),
    ));
    lines.extend([
        Line::from(""),
        Line::from(format!(
            "Frames: {}   Event batches: {}",
            bench.draws.count, bench.event_batches.count
        )),
        Line::from(format!(
            "Search: {}   Items: {}   Matches: {}",
            if !search_applicable {
                "N/A"
            } else if app.search_session.search.working {
                "working"
            } else {
                "idle"
            },
            app.search_session.search.total_items,
            app.search_session.search.total_matches
        )),
        Line::from(format!(
            "Terminal: {} × {}   Build: {}",
            f.area().width,
            f.area().height,
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        )),
        Line::from(""),
        Line::from(format!(
            "Timings use the latest {SAMPLE_LIMIT} samples per metric."
        )),
        Line::from("Draws include this modal; terminal display latency is excluded."),
        Line::from("Event timing excludes idle waits, draw time and background work."),
    ]);

    let full = f.area();
    let width = full.width.min(76);
    let height = full.height.min(lines.len() as u16 + 3);
    let area = Rect::new(
        full.x + (full.width - width) / 2,
        full.y + (full.height - height) / 2,
        width,
        height,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::LightCyan))
        .title(" Benchmarks ")
        .title_bottom(format!(
            " Esc / {} close · ↑/↓ scroll ",
            format_keybindings(&app.keybindings().toggle_bench)
        ));
    let inner = block.inner(area);
    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    let scroll = bench.scroll.min(max_scroll);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
    if let Some(bench) = app.ui.bench.as_mut() {
        bench.scroll = scroll;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::config::LoadedAppConfig;
    use clap::Parser;
    use ratatui::{backend::TestBackend, Terminal};

    fn app() -> App {
        let config =
            crate::cli::resolve_cli(Cli::parse_from(["binocular", "--bench"]), false).unwrap();
        App::from_configs(config.run, config.search, LoadedAppConfig::default())
    }

    fn screen(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn modal_renders_metrics_over_search_log_and_diff() {
        for mode in ["search", "log", "diff"] {
            let mut app = app();
            app.runtime.run.log = mode == "log";
            if mode == "diff" {
                app.runtime.run.diff = Some(["a".into(), "b".into()]);
            }
            let bench = app.ui.bench.as_mut().unwrap();
            bench.record_draw(Duration::from_millis(60));
            bench.event_batches.record(Duration::from_millis(2));
            let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
            terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let text = screen(&terminal);
            for expected in [
                "Benchmarks",
                "Slow drawing",
                "60.00 ms",
                "First frame",
                "F12",
                "Event batch processing",
            ] {
                assert!(text.contains(expected), "{mode}: missing {expected}");
            }
            app.apply_action(crate::app::AppAction::CloseBench);
            terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            assert!(!screen(&terminal).contains("Benchmarks"));
        }
    }

    #[test]
    fn small_modal_scrolls_and_clamps_after_resize() {
        let mut app = app();
        app.ui.bench.as_mut().unwrap().scroll = u16::MAX;
        let mut terminal = Terminal::new(TestBackend::new(76, 8)).unwrap();
        terminal.draw(|f| render_bench_modal(f, &mut app)).unwrap();
        assert!(screen(&terminal).contains("Event timing excludes"));
        assert!(app.ui.bench.as_ref().unwrap().scroll < u16::MAX);
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal.draw(|f| render_bench_modal(f, &mut app)).unwrap();
        assert_eq!(app.ui.bench.as_ref().unwrap().scroll, 0);
        for (width, height) in [(0, 0), (1, 1), (10, 3)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| render_bench_modal(f, &mut app)).unwrap();
        }
    }
}
