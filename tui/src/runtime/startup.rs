use crate::infra::channel::Sender;
use crate::preview::structured_log;
use crate::preview::structured_log::{parse_line, LogEntry, LogFormat};
use crate::runtime::config::RunConfig;
use std::io::{self, BufRead, Read};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupMode {
    Headless,
    InteractiveDirectDiff,
    InteractiveTerminal,
    InteractiveSearchPipe,
    InteractiveLogPipe,
    InteractiveLogFile,
}

pub type LogPipeReader = Box<dyn Read + Send>;

pub struct PreparedInteractiveInput {
    pub stdin_items: Option<Vec<String>>,
    pub log_pipe: Option<LogPipeReader>,
    pub log_files: Vec<PathBuf>,
}

pub fn classify_input_mode_with_run_config(run_config: &RunConfig) -> StartupMode {
    if run_config.headless {
        StartupMode::Headless
    } else if run_config.diff.is_some() {
        StartupMode::InteractiveDirectDiff
    } else if run_config.log && !run_config.log_files.is_empty() {
        StartupMode::InteractiveLogFile
    } else if run_config.log && run_config.stdin {
        StartupMode::InteractiveLogPipe
    } else if run_config.stdin {
        StartupMode::InteractiveSearchPipe
    } else {
        StartupMode::InteractiveTerminal
    }
}

pub fn prepare_headless_input_with_run_config(
    run_config: &RunConfig,
) -> anyhow::Result<Option<Vec<String>>> {
    if !run_config.stdin {
        return Ok(None);
    }

    let raw = read_stdin_lines()?;
    Ok(Some(parse_stdin_items(raw, run_config.split.as_deref())))
}

pub fn prepare_interactive_input_with_run_config(
    run_config: &RunConfig,
) -> anyhow::Result<PreparedInteractiveInput> {
    match classify_input_mode_with_run_config(run_config) {
        StartupMode::Headless
        | StartupMode::InteractiveDirectDiff
        | StartupMode::InteractiveTerminal => Ok(PreparedInteractiveInput {
            stdin_items: None,
            log_pipe: None,
            log_files: vec![],
        }),
        StartupMode::InteractiveSearchPipe => Ok(PreparedInteractiveInput {
            stdin_items: Some(read_interactive_search_items(run_config.split.as_deref())?),
            log_pipe: None,
            log_files: vec![],
        }),
        StartupMode::InteractiveLogPipe => Ok(PreparedInteractiveInput {
            stdin_items: None,
            log_pipe: Some(take_interactive_log_pipe()?),
            log_files: vec![],
        }),
        StartupMode::InteractiveLogFile => Ok(PreparedInteractiveInput {
            stdin_items: None,
            log_pipe: None,
            log_files: run_config.log_files.clone(),
        }),
    }
}

pub fn spawn_log_stdin_reader(
    pipe: LogPipeReader,
    tx_log: impl Sender<(String, Vec<LogEntry>)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut format: Option<LogFormat> = None;
        let mut batch: Vec<LogEntry> = Vec::with_capacity(256);
        let mut last_flush = std::time::Instant::now();
        const BATCH_SIZE: usize = 500;
        const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

        fn flush(
            batch: &mut Vec<LogEntry>,
            tx: &impl Sender<(String, Vec<LogEntry>)>,
            last_flush: &mut std::time::Instant,
        ) -> bool {
            if batch.is_empty() {
                return true;
            }
            let ok = tx
                .send((
                    structured_log::STDIN_STREAM_PATH.to_string(),
                    std::mem::replace(batch, Vec::with_capacity(256)),
                ))
                .is_ok();
            *last_flush = std::time::Instant::now();
            ok
        }

        for line in std::io::BufReader::new(pipe).lines() {
            let Ok(line) = line else { break };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if format.is_none() {
                format = Some(
                    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                        LogFormat::Jsonl
                    } else {
                        LogFormat::Logfmt
                    },
                );
            }
            if let Some(entry) = parse_line(trimmed, format.as_ref().expect("log format set")) {
                batch.push(entry);
                if batch.len() >= BATCH_SIZE || last_flush.elapsed() >= FLUSH_INTERVAL {
                    if !flush(&mut batch, &tx_log, &mut last_flush) {
                        break; // Receiver gone, app is closing.
                    }
                }
            }
        }

        let _ = flush(&mut batch, &tx_log, &mut last_flush);
    })
}

pub fn spawn_log_file_watchers(
    files: &[PathBuf],
    tx_log: impl Sender<(String, Vec<LogEntry>)> + Clone + 'static,
) {
    let format = detect_format_from_files(files).unwrap_or(LogFormat::Jsonl);
    for file in files {
        let stop = Arc::new(AtomicBool::new(false));
        structured_log::watcher::spawn_log_watcher(
            file.display().to_string(),
            format.clone(),
            0, // start from beginning (read whole file, then tail)
            stop,
            tx_log.clone(),
        );
    }
}

fn detect_format_from_files(files: &[PathBuf]) -> Option<LogFormat> {
    for file in files {
        let file = std::fs::File::open(file).ok()?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            return Some(
                if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                    LogFormat::Jsonl
                } else {
                    LogFormat::Logfmt
                },
            );
        }
    }
    None
}

fn read_interactive_search_items(split: Option<&str>) -> anyhow::Result<Vec<String>> {
    let raw = read_piped_stdin_lines_with_tty_restore()?;
    Ok(parse_stdin_items(raw, split))
}

#[cfg(unix)]
fn take_interactive_log_pipe() -> anyhow::Result<LogPipeReader> {
    let pipe = std::fs::File::open("/dev/stdin")?;
    restore_stdin_to_real_tty()?;
    Ok(Box::new(pipe))
}

#[cfg(not(unix))]
fn take_interactive_log_pipe() -> anyhow::Result<LogPipeReader> {
    Ok(Box::new(std::io::stdin()))
}

#[cfg(unix)]
fn read_piped_stdin_lines_with_tty_restore() -> anyhow::Result<Vec<String>> {
    let pipe_stdin = std::fs::File::open("/dev/stdin")?;
    let items = read_lines_from(pipe_stdin);
    restore_stdin_to_real_tty()?;
    Ok(items)
}

#[cfg(not(unix))]
fn read_piped_stdin_lines_with_tty_restore() -> anyhow::Result<Vec<String>> {
    read_stdin_lines()
}

#[cfg(unix)]
fn restore_stdin_to_real_tty() -> anyhow::Result<()> {
    use std::os::fd::AsRawFd;

    let tty = open_real_tty()?;
    let ret = unsafe { libc::dup2(tty.as_raw_fd(), libc::STDIN_FILENO) };
    if ret == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
fn open_real_tty() -> anyhow::Result<std::fs::File> {
    use std::ffi::CStr;

    for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO] {
        let name = unsafe { libc::ttyname(fd) };
        if !name.is_null() {
            let path = unsafe { CStr::from_ptr(name) };
            if let Ok(path_str) = path.to_str() {
                if let Ok(file) = std::fs::File::open(path_str) {
                    return Ok(file);
                }
            }
        }
    }

    Ok(std::fs::File::open("/dev/tty")?)
}

fn read_stdin_lines() -> anyhow::Result<Vec<String>> {
    Ok(read_lines_from(io::stdin().lock()))
}

fn read_lines_from(reader: impl Read) -> Vec<String> {
    io::BufReader::new(reader)
        .lines()
        .map_while(Result::ok)
        .collect()
}

fn parse_stdin_items(raw: Vec<String>, split: Option<&str>) -> Vec<String> {
    raw.into_iter()
        .flat_map(|line| match split {
            Some(delim) => line.split(delim).map(str::to_string).collect::<Vec<_>>(),
            None => vec![line],
        })
        .filter(|s| !s.trim().is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::OutputFormat;
    use crate::runtime::config::RunConfig;

    fn run_config() -> RunConfig {
        RunConfig {
            headless: false,
            output_format: OutputFormat::Plain,
            stdin: false,
            log: false,
            log_files: vec![],
            diff: None,
            preview_command: None,
            preview_delimiter: ":".to_string(),
            split: None,
        }
    }

    #[test]
    fn headless_mode_classification_is_stable() {
        let mut run_config = run_config();
        run_config.headless = true;
        run_config.stdin = true;
        assert_eq!(
            classify_input_mode_with_run_config(&run_config),
            StartupMode::Headless
        );
    }

    #[test]
    fn piped_stdin_search_mode_classification_is_stable() {
        let mut run_config = run_config();
        run_config.stdin = true;
        assert_eq!(
            classify_input_mode_with_run_config(&run_config),
            StartupMode::InteractiveSearchPipe
        );
    }

    #[test]
    fn piped_stdin_log_mode_classification_is_stable() {
        let mut run_config = run_config();
        run_config.stdin = true;
        run_config.log = true;
        assert_eq!(
            classify_input_mode_with_run_config(&run_config),
            StartupMode::InteractiveLogPipe
        );
    }

    #[test]
    fn direct_diff_mode_classification_is_stable() {
        let mut run_config = run_config();
        run_config.diff = Some(["left.txt".into(), "right.txt".into()]);
        assert_eq!(
            classify_input_mode_with_run_config(&run_config),
            StartupMode::InteractiveDirectDiff
        );
    }

    #[test]
    fn split_parsing_drops_empty_segments() {
        let items = parse_stdin_items(
            vec!["a,b,,c".to_string(), "  ".to_string(), "d".to_string()],
            Some(","),
        );
        assert_eq!(items, vec!["a", "b", "c", "d"]);
    }
}
