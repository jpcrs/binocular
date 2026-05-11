use crate::preview::worker::executor::PreviewExecution;
use crate::preview::{
    apply_param_substitutions, PreviewContent, PreviewRequest, PREVIEW_COMMAND_POLL_INTERVAL,
    PREVIEW_COMMAND_TIMEOUT,
};
use ratatui::text::Text;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

pub(crate) fn execute_preview_command<F>(
    request: PreviewRequest,
    item: &str,
    command: &str,
    delimiter: &str,
    poll_replacement: &mut F,
) -> PreviewExecution
where
    F: FnMut() -> Option<PreviewRequest>,
{
    let Some(argv) = split_command(command) else {
        return PreviewExecution::Completed(
            request,
            PreviewContent::PlainText(Text::from("Invalid --preview command")),
        );
    };

    let Some(program) = argv.first() else {
        return PreviewExecution::Completed(
            request,
            PreviewContent::PlainText(Text::from("Empty --preview command")),
        );
    };

    let parts: Vec<&str> = item.split(delimiter).collect();
    let has_placeholder = argv.iter().skip(1).any(|a| a.contains('{'));

    let mut cmd = Command::new(program);
    for arg in argv.iter().skip(1) {
        cmd.arg(apply_param_substitutions(arg, item, &parts));
    }

    if !has_placeholder {
        cmd.stdin(Stdio::piped());
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.env("BINOCULAR_PREVIEW_ITEM", item);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return PreviewExecution::Completed(
                request,
                PreviewContent::PlainText(Text::from(format!(
                    "Failed to start preview command: {}",
                    err
                ))),
            );
        }
    };

    if !has_placeholder {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(item.as_bytes());
        }
    }

    let started_at = Instant::now();
    loop {
        if let Some(next_request) = poll_replacement() {
            let _ = child.kill();
            let _ = child.wait();
            return PreviewExecution::Superseded(next_request);
        }

        if started_at.elapsed() >= PREVIEW_COMMAND_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return PreviewExecution::Completed(
                request,
                PreviewContent::PlainText(Text::from(format!(
                    "Preview command timed out after {}s",
                    PREVIEW_COMMAND_TIMEOUT.as_secs()
                ))),
            );
        }

        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(PREVIEW_COMMAND_POLL_INTERVAL),
            Err(err) => {
                return PreviewExecution::Completed(
                    request,
                    PreviewContent::PlainText(Text::from(format!(
                        "Failed to poll preview command: {}",
                        err
                    ))),
                );
            }
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(err) => {
            return PreviewExecution::Completed(
                request,
                PreviewContent::PlainText(Text::from(format!(
                    "Failed to read preview command output: {}",
                    err
                ))),
            );
        }
    };

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&stderr);
    }

    if text.trim().is_empty() {
        text = if output.status.success() {
            "Preview command produced no output".to_string()
        } else {
            format!("Preview command exited with status {}", output.status)
        };
    }

    PreviewExecution::Completed(request, PreviewContent::PlainText(Text::from(text)))
}

/// Split a string into words using basic shell-like quoting rules.
///
/// Supports:
/// - Single quotes: `'...'`
/// - Double quotes: `"..."`
/// - Backslash escapes outside quotes and inside double quotes
///
/// Returns `None` for unmatched quotes or a trailing unescaped backslash.
fn split_command(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_word = false;

    while let Some(ch) = chars.next() {
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current.push(ch);
            }
            in_word = true;
        } else if in_double {
            if ch == '\\' {
                match chars.next() {
                    Some(c @ ('"' | '\\' | '$' | '`')) => current.push(c),
                    Some('\n') => {}
                    Some(next) => {
                        current.push('\\');
                        current.push(next);
                    }
                    None => return None,
                }
            } else if ch == '"' {
                in_double = false;
            } else {
                current.push(ch);
            }
            in_word = true;
        } else {
            match ch {
                '\\' => {
                    match chars.next() {
                        Some(next) => current.push(next),
                        None => return None,
                    }
                    in_word = true;
                }
                '\'' => {
                    in_single = true;
                    in_word = true;
                }
                '"' => {
                    in_double = true;
                    in_word = true;
                }
                c if c.is_whitespace() => {
                    if in_word {
                        words.push(std::mem::take(&mut current));
                        in_word = false;
                    }
                }
                _ => {
                    current.push(ch);
                    in_word = true;
                }
            }
        }
    }

    if in_single || in_double {
        return None;
    }

    if in_word {
        words.push(current);
    }

    Some(words)
}
