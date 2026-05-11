pub mod archive;
pub mod binary;
pub mod diff;
pub mod directory;
pub mod doc;
pub mod encoding;
pub mod image;
pub mod media;
pub mod pdf;
pub mod protocol;
pub mod request;
pub mod rich_text;
pub mod sqlite;
pub mod structured_log;
pub mod types;
pub mod worker;

use ratatui_image::picker::Picker;
use std::time::Duration;

const PREVIEW_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PREVIEW_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

/// Spawn the preview worker thread.
///
/// This function spawns a background thread that receives file paths and
/// generates preview content, sending results back through the provided channel.
pub fn spawn_previewer(
    rx_request: impl crate::infra::channel::Receiver<PreviewRequest> + 'static,
    tx_preview: impl crate::infra::channel::Sender<(PreviewSource, PreviewContent)> + 'static,
    tx_log: impl crate::infra::channel::Sender<(String, Vec<LogEntry>)> + 'static,
    picker: Picker,
    preview_command: Option<String>,
    delimiter: String,
    log_max_entries: usize,
) {
    std::thread::spawn(move || {
        // SECURITY: isolate the preview worker so a panic in any parsing dependency
        // (image, PDF, ZIP, tree-sitter, etc.) cannot crash the entire application.
        // This requires panic = "unwind" in the release profile.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let executor = worker::executor::PreviewExecutor::new(
                picker,
                preview_command,
                delimiter,
                log_max_entries,
            );
            let mut orchestrator = worker::orchestrator::PreviewOrchestrator::new(
                rx_request, tx_preview, tx_log, executor,
            );
            orchestrator.run();
        }));

        if let Err(panic_info) = result {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic in preview worker".to_string()
            };
            eprintln!("Preview worker panicked: {}", msg);
        }
    });
}

fn build_path_preview(path_str: &str, picker: &Picker, log_max_entries: usize) -> PreviewContent {
    request::path::registry::build_path_preview(path_str, picker, log_max_entries)
}

/// Applies `{}` and `{N}` placeholder substitutions in a single command argument.
///
/// - `{N}` is replaced with `parts[N]` (item split by delimiter).
/// - `{}` is replaced with the full item string.
///
/// # Security
/// Substitutions are raw string replacements with **no shell escaping**. If the
/// user wraps their preview command in a shell (e.g. `sh -c "echo {}"`), shell
/// metacharacters in the selected item can be interpreted by the shell. This is
/// by design — we pass arguments directly to `std::process::Command` without a
/// shell wrapper. Only use shell wrappers with trusted input.
pub(super) fn apply_param_substitutions(arg: &str, item: &str, parts: &[&str]) -> String {
    let mut result = arg.to_string();
    for (i, part) in parts.iter().enumerate() {
        result = result.replace(&format!("{{{i}}}"), part);
    }
    result.replace("{}", item)
}

pub use protocol::{PreviewRequest, PreviewSource};
pub use rich_text::syntax::{
    detect_language, get_configs, get_highlighter, get_style, SyntaxRegistry,
};
pub use rich_text::RichTextDocument;
pub use rich_text::{
    apply_text_edit, create_rich_text_document, edit_content_delete_char,
    edit_content_delete_char_at, edit_content_delete_range, edit_content_insert_char,
    edit_content_insert_text, generate_plain_lines_for_range, regenerate_lines,
};
pub use structured_log::LogEntry;
pub use types::{DiffPreview, ImagePreview, LogPreview, MediaPreview, PreviewContent};
