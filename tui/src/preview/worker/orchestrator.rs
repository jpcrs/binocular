use super::executor::{PreviewExecution, PreviewExecutor};
use crate::infra::channel::{Receiver, Sender};
use crate::preview::{structured_log, LogEntry, PreviewContent, PreviewRequest, PreviewSource};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub(crate) struct PreviewOrchestrator<R, SP, SL> {
    rx_request: R,
    tx_preview: SP,
    executor: PreviewExecutor,
    log_watch_service: LogWatchService<SL>,
}

impl<R, SP, SL> PreviewOrchestrator<R, SP, SL>
where
    R: Receiver<PreviewRequest>,
    SP: Sender<(PreviewSource, PreviewContent)>,
    SL: Sender<(String, Vec<LogEntry>)>,
{
    pub(crate) fn new(
        rx_request: R,
        tx_preview: SP,
        tx_log: SL,
        executor: PreviewExecutor,
    ) -> Self {
        Self {
            rx_request,
            tx_preview,
            executor,
            log_watch_service: LogWatchService::new(tx_log),
        }
    }

    pub(crate) fn run(&mut self) {
        while let Ok(first_request) = self.rx_request.recv() {
            self.log_watch_service.stop();
            let mut active_request = self.drain_pending_requests(first_request);

            loop {
                match self
                    .executor
                    .execute(active_request, || self.take_newest_pending_request())
                {
                    PreviewExecution::Completed(request, preview) => {
                        self.publish_completed_preview(&request, preview);
                        break;
                    }
                    PreviewExecution::Superseded(newer_request) => {
                        active_request = self.drain_pending_requests(newer_request);
                    }
                }
            }
        }

        self.log_watch_service.stop();
    }

    fn drain_pending_requests(&self, mut newest_request: PreviewRequest) -> PreviewRequest {
        while let Ok(Some(next_request)) = self.rx_request.try_recv() {
            newest_request = next_request;
        }
        newest_request
    }

    fn take_newest_pending_request(&self) -> Option<PreviewRequest> {
        let mut newest_request = None;
        while let Ok(Some(next_request)) = self.rx_request.try_recv() {
            newest_request = Some(next_request);
        }
        newest_request
    }

    fn publish_completed_preview(&mut self, request: &PreviewRequest, preview: PreviewContent) {
        self.log_watch_service.replace_for(request, &preview);
        let _ = self.tx_preview.send((request.source().clone(), preview));
    }
}

struct LogWatchService<S> {
    tx_log: S,
    current_stop: Option<Arc<AtomicBool>>,
}

impl<S> LogWatchService<S>
where
    S: Sender<(String, Vec<LogEntry>)>,
{
    fn new(tx_log: S) -> Self {
        Self {
            tx_log,
            current_stop: None,
        }
    }

    fn replace_for(&mut self, request: &PreviewRequest, preview: &PreviewContent) {
        self.stop();

        let PreviewContent::StructuredLog(log_preview) = preview else {
            return;
        };
        let Some(path) = request.file_path() else {
            return;
        };
        let Ok(metadata) = fs::metadata(path) else {
            return;
        };

        let stop = Arc::new(AtomicBool::new(false));
        structured_log::watcher::spawn_log_watcher(
            path.to_string(),
            log_preview.log.format.clone(),
            metadata.len(),
            stop.clone(),
            self.tx_log.clone(),
        );
        self.current_stop = Some(stop);
    }

    fn stop(&mut self) {
        if let Some(stop) = self.current_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

impl<S> Drop for LogWatchService<S> {
    fn drop(&mut self) {
        if let Some(stop) = self.current_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::channel;
    use crate::search::types::SearchItem;
    use ratatui_image::picker::Picker;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str, ext: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("binocular-{name}-{nanos}.{ext}"))
    }

    fn recv_with_timeout<T>(rx: &channel::DefaultReceiver<T>, timeout: Duration) -> Option<T>
    where
        T: Send + 'static,
    {
        let started = std::time::Instant::now();
        loop {
            match rx.try_recv() {
                Ok(Some(value)) => return Some(value),
                Ok(None) if started.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }

    #[test]
    fn rapid_preview_replacement_only_emits_latest_request() {
        let (tx_request, rx_request) = channel::unbounded_default::<PreviewRequest>();
        let (tx_preview, rx_preview) =
            channel::unbounded_default::<(PreviewSource, PreviewContent)>();
        let (tx_log, _rx_log) = channel::unbounded_default::<(String, Vec<LogEntry>)>();

        let executor = PreviewExecutor::new(
            Picker::halfblocks(),
            Some("sh -c 'sleep 1'".to_string()),
            ":".to_string(),
            100_000,
        );
        let mut orchestrator = PreviewOrchestrator::new(rx_request, tx_preview, tx_log, executor);

        let handle = std::thread::spawn(move || orchestrator.run());

        let first = PreviewRequest::StdinOrCommand {
            source: PreviewSource::SearchItem(SearchItem::stdin("first")),
            item: "first".to_string(),
        };
        let second = PreviewRequest::StdinOrCommand {
            source: PreviewSource::SearchItem(SearchItem::stdin("second")),
            item: "second".to_string(),
        };
        tx_request.send(first).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        tx_request.send(second.clone()).unwrap();

        let (source, _) =
            recv_with_timeout(&rx_preview, Duration::from_secs(3)).expect("preview response");
        assert_eq!(source, second.source().clone());

        drop(tx_request);
        handle.join().unwrap();
    }

    #[test]
    fn structured_log_watcher_starts_and_stops_with_active_preview() {
        let path = unique_temp_path("watcher", "jsonl");
        fs::write(&path, "{\"level\":\"info\",\"msg\":\"start\"}\n").unwrap();

        let (tx_request, rx_request) = channel::unbounded_default::<PreviewRequest>();
        let (tx_preview, rx_preview) =
            channel::unbounded_default::<(PreviewSource, PreviewContent)>();
        let (tx_log, rx_log) = channel::unbounded_default::<(String, Vec<LogEntry>)>();

        let executor = PreviewExecutor::new(Picker::halfblocks(), None, ":".to_string(), 100_000);
        let mut orchestrator = PreviewOrchestrator::new(rx_request, tx_preview, tx_log, executor);
        let handle = std::thread::spawn(move || orchestrator.run());

        let path_string = path.display().to_string();
        let request = PreviewRequest::Path {
            source: PreviewSource::SearchItem(SearchItem::path(path_string.clone())),
            path: path_string.clone(),
        };
        tx_request.send(request).unwrap();

        let (_, preview) =
            recv_with_timeout(&rx_preview, Duration::from_secs(2)).expect("initial preview");
        assert!(matches!(preview, PreviewContent::StructuredLog(_)));

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            writeln!(file, "{{\"level\":\"info\",\"msg\":\"append-1\"}}").unwrap();
        }

        let (_, entries) =
            recv_with_timeout(&rx_log, Duration::from_secs(2)).expect("watcher append");
        assert_eq!(entries.len(), 1);

        let replacement = PreviewRequest::StdinOrCommand {
            source: PreviewSource::SearchItem(SearchItem::stdin("replacement")),
            item: "replacement".to_string(),
        };
        tx_request.send(replacement).unwrap();
        let _ =
            recv_with_timeout(&rx_preview, Duration::from_secs(2)).expect("replacement preview");

        while rx_log.try_recv().unwrap_or(None).is_some() {}

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            writeln!(file, "{{\"level\":\"info\",\"msg\":\"append-2\"}}").unwrap();
        }

        assert!(recv_with_timeout(&rx_log, Duration::from_millis(700)).is_none());

        drop(tx_request);
        handle.join().unwrap();
        let _ = fs::remove_file(path);
    }
}
