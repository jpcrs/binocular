//! Reproducible CPU-side benchmarks. Run each case in a fresh release process.
//! cargo run --release -p binocular-cli --example performance -- preview rs
//! cargo run --release -p binocular-cli --example performance -- draw
//! cargo run --release -p binocular-cli --example performance -- matcher
use binocular::{
    app::{InputMode, Mode},
    infra::channel::{self, Receiver, Sender},
    preview::{self, PreviewContent},
    search::{
        matcher::{spawn_matcher, MatcherCommand},
        types::SearchItem,
    },
    ui::preview::{render_preview, PreviewView},
};
use ratatui::{
    backend::TestBackend,
    style::{Color, Style},
    text::{Line, Span, Text},
    Terminal,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hash::{Hash, Hasher},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct CountingAllocator;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn checksum(value: impl std::fmt::Debug) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    format!("{value:?}").hash(&mut hash);
    hash.finish()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("preview") => {
            let ext = args.get(2).map(String::as_str).unwrap_or("rs");
            let source = match ext {
                "rs" => "fn main() { let café = 42; println!(\"hello {}\", café); }\n",
                "py" => "def greet(name):\n    return f'hello {name}' # greeting\n",
                "js" | "ts" => "const greet = (name) => `hello ${name}`; // greeting\n",
                "json" => "{\"message\": \"hello\", \"count\": 42}\n",
                "toml" => "[service]\nname = \"hello\"\ncount = 42\n",
                "yaml" => "service:\n  name: hello\n  count: 42\n",
                "html" => "<div class=\"hello\">hello <b>world</b></div>\n",
                "css" => ".hello { color: red; margin: 42px; }\n",
                "c" | "cpp" => "int main() { /* hello */ return 42; }\n",
                "go" => "package main\nfunc main() { println(\"hello\") }\n",
                "cs" => "class Hello { static void Main() { int count = 42; } }\n",
                _ => "hello world\n",
            }
            .repeat(100);
            let start = Instant::now();
            let document =
                preview::create_rich_text_document(source, Path::new(&format!("sample.{ext}")));
            let elapsed = start.elapsed();
            println!(
                "preview {ext}: {:.3} ms; lines {}; checksum {}",
                elapsed.as_secs_f64() * 1000.,
                document.line_count(),
                checksum((
                    &document.lines,
                    document
                        .tree
                        .as_ref()
                        .map(|tree| tree.root_node().to_sexp())
                ))
            );
        }
        Some("draw") => {
            let text = Text::from(
                (0..50_000)
                    .map(|i| {
                        Line::from(vec![
                            Span::styled(format!("{i:05} + "), Style::default().fg(Color::Green)),
                            Span::raw("let café = \"你好 👩‍💻\"; // a styled diff line".to_string()),
                        ])
                    })
                    .collect::<Vec<_>>(),
            );
            let mut content = PreviewContent::PlainText(text);
            let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
            let view = PreviewView {
                app_mode: Mode::Preview,
                preview_mode: InputMode::Normal,
                source: None,
                status_message: None,
                command_buffer: None,
                highlight_line: None,
                search_query: "",
                selection_start: None,
                cursor_line: 0,
                cursor_char: 0,
                scroll: 40_000,
                scroll_char: 3,
                area_height: 40,
            };
            // Warm up Ratatui's layout cache.
            terminal
                .draw(|f| render_preview(f, &view, Some(&mut content), f.area()))
                .unwrap();
            let start = Instant::now();
            for _ in 0..100 {
                terminal
                    .draw(|f| render_preview(f, &view, Some(&mut content), f.area()))
                    .unwrap();
            }
            let elapsed = start.elapsed();
            // Count a separate draw so atomic increments do not skew the timing.
            COUNTING.store(true, Ordering::Relaxed);
            terminal
                .draw(|f| render_preview(f, &view, Some(&mut content), f.area()))
                .unwrap();
            COUNTING.store(false, Ordering::Relaxed);
            println!(
                "draw: {:.3} ms/frame; allocations/frame {}; checksum {}",
                elapsed.as_secs_f64() * 10.,
                ALLOCATIONS.load(Ordering::Relaxed),
                checksum(terminal.backend().buffer())
            );
        }
        Some("matcher") => {
            let (tx_items, rx_items) = channel::unbounded_default();
            let (tx_cmd, rx_cmd) = channel::unbounded_default();
            let (tx_state, rx_state) = channel::unbounded_default();
            let stop = Arc::new(AtomicBool::new(false));
            let worker = spawn_matcher(rx_items, rx_cmd, stop.clone(), tx_state, false, false);
            tx_cmd.send(MatcherCommand::Resize(5000)).unwrap();
            tx_cmd
                .send(MatcherCommand::Query("service".into()))
                .unwrap();
            tx_items
                .send(
                    (0..20_000)
                        .map(|i| SearchItem::path(format!("src/service_{i:05}/module.rs")))
                        .collect(),
                )
                .unwrap();
            drop(tx_items);
            let start = Instant::now();
            let mut rows = Vec::new();
            loop {
                let mut state = rx_state.recv().unwrap();
                state.apply_results(&mut rows);
                if !state.working && state.total_items == 20_000 {
                    assert_eq!(state.total_matches, 20_000);
                    assert_eq!(rows.len(), 5000);
                    println!(
                        "matcher load: {:.3} ms; checksum {}",
                        start.elapsed().as_secs_f64() * 1000.,
                        checksum(
                            rows.iter()
                                .map(|r| (&r.item, &r.indices, r.column))
                                .collect::<Vec<_>>()
                        )
                    );
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(10));
            }
            std::thread::sleep(Duration::from_millis(100));
            COUNTING.store(true, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(500));
            COUNTING.store(false, Ordering::Relaxed);
            println!(
                "matcher idle: {} allocations/500 ms",
                ALLOCATIONS.load(Ordering::Relaxed)
            );
            stop.store(true, Ordering::Relaxed);
            worker.join().unwrap();
        }
        _ => panic!("expected preview [extension], draw, or matcher"),
    }
}
