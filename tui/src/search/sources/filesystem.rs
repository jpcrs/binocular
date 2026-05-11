use crate::infra::channel::{BatchSender, Sender};
use crate::search::types::{SearchConfig, SearchItem};
use ignore::WalkBuilder;
use std::io::{BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Directories to ignore during file traversal.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "build",
    "dist",
    "out",
    "node_modules",
    "vendor",
    "bower_components",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".eggs",
    "*.egg-info",
    ".idea",
    ".cache",
    ".parcel-cache",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".gradle",
    "coverage",
];

/// Maximum line length to read during content search (1 MiB).
/// Lines longer than this are truncated to prevent memory exhaustion.
const MAX_GREP_LINE_LENGTH: usize = 1024 * 1024;

pub fn spawn_searcher_with_config(
    config: SearchConfig,
    stop: Arc<AtomicBool>,
    tx: impl Sender<Vec<SearchItem>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(2);

        let mut roots = config.locations.iter();
        let first = roots
            .next()
            .map(|p| p.as_path())
            .unwrap_or(std::path::Path::new("."));
        let mut builder = WalkBuilder::new(first);
        for extra in roots {
            builder.add(extra);
        }
        builder
            .hidden(config.no_hidden)
            .git_ignore(!config.no_git_ignore)
            .ignore(!config.no_ignore)
            .threads(threads);
        if !config.no_default_ignore_dirs {
            builder.filter_entry(|entry| {
                let name = entry.file_name().to_str().unwrap_or("");
                !IGNORED_DIRS.contains(&name)
            });
        }
        let walker = builder.build_parallel();

        walker.run(|| {
            let mut batcher = BatchSender::new(tx.clone(), 256);
            let config = config.clone();
            let stop = stop.clone();

            Box::new(move |result| {
                use ignore::WalkState;

                if stop.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }

                if let Ok(entry) = result {
                    let file_type = entry.file_type();
                    let matches_kind = match file_type {
                        Some(t) if config.settings.mode.is_dir_only() => t.is_dir(),
                        Some(t) => t.is_file(),
                        None => false,
                    };

                    if matches_kind {
                        let path = entry.path();

                        if config.settings.mode.is_content() {
                            if config.search_pdf
                                && path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|e| e.eq_ignore_ascii_case("pdf"))
                                    .unwrap_or(false)
                            {
                                if let Ok(lines) = crate::preview::pdf::extract_all_text(path) {
                                    let path_str = path.to_string_lossy();
                                    for (ln, line) in lines.iter().enumerate() {
                                        if stop.load(Ordering::Relaxed) {
                                            return WalkState::Quit;
                                        }
                                        batcher.push(SearchItem::grep(
                                            path_str.as_ref(),
                                            ln + 1,
                                            line,
                                        ));
                                    }
                                }
                            } else if let Ok(file) = std::fs::File::open(path) {
                                let path_str = path.to_string_lossy();
                                let mut reader = BufReader::with_capacity(64 * 1024, file);
                                let mut raw = Vec::with_capacity(256);
                                let mut ln = 0usize;
                                loop {
                                    if ln % 128 == 0 && stop.load(Ordering::Relaxed) {
                                        return WalkState::Quit;
                                    }
                                    raw.clear();
                                    match reader.read_until(b'\n', &mut raw) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if raw.len() > MAX_GREP_LINE_LENGTH {
                                                raw.truncate(MAX_GREP_LINE_LENGTH);
                                                // Drain the rest of this line without allocating.
                                                let mut discard = [0u8; 8192];
                                                loop {
                                                    match reader.read(&mut discard) {
                                                        Ok(0) => break,
                                                        Ok(d) => {
                                                            if discard[..d].contains(&b'\n') {
                                                                break;
                                                            }
                                                        }
                                                        Err(_) => break,
                                                    }
                                                }
                                            }
                                            if memchr::memchr(b'\x00', &raw).is_some() {
                                                break;
                                            }
                                            if raw.last() == Some(&b'\n') {
                                                raw.pop();
                                            }
                                            if raw.last() == Some(&b'\r') {
                                                raw.pop();
                                            }
                                            if !raw.is_empty() {
                                                if let Ok(s) = std::str::from_utf8(&raw) {
                                                    batcher.push(SearchItem::grep(
                                                        path_str.as_ref(),
                                                        ln + 1,
                                                        s,
                                                    ));
                                                }
                                            }
                                            ln += 1;

                                            if n > MAX_GREP_LINE_LENGTH {
                                                ln += 1;
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                        } else {
                            batcher.push(SearchItem::path(path.to_string_lossy()));
                        }
                    } else {
                        batcher.tick();
                    }
                }
                ignore::WalkState::Continue
            })
        });
    })
}
