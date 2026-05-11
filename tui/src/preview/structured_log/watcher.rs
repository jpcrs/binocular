use super::{parse_line, LogEntry, LogFormat};
use crate::infra::channel::Sender;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const POLL_MS: Duration = Duration::from_millis(250);

pub fn spawn_log_watcher(
    path: String,
    format: LogFormat,
    start_offset: u64,
    stop: Arc<AtomicBool>,
    tx: impl Sender<(String, Vec<LogEntry>)> + 'static,
) {
    std::thread::spawn(move || {
        let Ok(file) = std::fs::File::open(&path) else {
            return;
        };
        let mut reader = BufReader::new(file);
        let _ = reader.seek(SeekFrom::Start(start_offset));

        let mut line_buf = String::new();

        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            std::thread::sleep(POLL_MS);

            if stop.load(Ordering::Relaxed) {
                break;
            }

            let mut new_entries: Vec<LogEntry> = Vec::new();

            loop {
                line_buf.clear();
                match reader.read_line(&mut line_buf) {
                    Ok(0) => break, // EOF. no new data yet
                    Ok(_) => {
                        if line_buf.ends_with('\n') {
                            // Complete line
                            if let Some(entry) = parse_line(line_buf.trim_end(), &format) {
                                new_entries.push(entry);
                            }
                        } else {
                            // Incomplete line, seek back and wait for more data.
                            let back = line_buf.len() as i64;
                            let _ = reader.seek(SeekFrom::Current(-back));
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            if !new_entries.is_empty() {
                if tx.send((path.clone(), new_entries)).is_err() {
                    break; // Receiver gone, app is closing.
                }
            }
        }
    });
}
