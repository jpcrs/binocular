use crate::infra::channel::{self, Receiver, Sender};
use crate::output::format_item_output;
use crate::runtime::config::RunConfig;
use crate::search::matcher::{spawn_exact_matcher, spawn_matcher, MatcherCommand, MatcherState};
use crate::search::sources::{
    spawn_git_searcher, spawn_searcher_with_config, spawn_stdin_searcher,
};
use crate::search::types::{SearchConfig, SearchItem, SearchResult};
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn run_with_configs(
    run_config: RunConfig,
    search_config: SearchConfig,
    stdin_items: Option<Vec<String>>,
) -> anyhow::Result<()> {
    debug_assert!(run_config.headless);
    run_with_search_config(search_config, stdin_items)
}

pub fn run_with_search_config(
    search_config: SearchConfig,
    stdin_items: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let query = search_config
        .query
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if query.is_empty() {
        stream_search_results(search_config, stdin_items)
    } else {
        stream_matched_results(search_config, stdin_items, query)
    }
}

fn stream_search_results(
    search_config: SearchConfig,
    stdin_items: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let (tx_items, rx_items) = channel::unbounded_default::<Vec<SearchItem>>();
    let stop = Arc::new(AtomicBool::new(false));
    if let Some(scope) = search_config.git_search_scope.clone() {
        let _ = spawn_git_searcher(scope, stop, tx_items);
    } else if let Some(items) = stdin_items {
        let _ = spawn_stdin_searcher(items, stop, tx_items);
    } else {
        let _ = spawn_searcher_with_config(search_config, stop, tx_items);
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    while let Ok(batch) = rx_items.recv() {
        for item in batch {
            writeln!(out, "{}", format_item_output(&item, None, false))?;
        }
    }

    Ok(())
}

fn stream_matched_results(
    search_config: SearchConfig,
    stdin_items: Option<Vec<String>>,
    query: String,
) -> anyhow::Result<()> {
    let (tx_items, rx_items) = channel::unbounded_default::<Vec<SearchItem>>();
    let (tx_cmd, rx_cmd) = channel::unbounded_default::<MatcherCommand>();
    let (tx_state, rx_state) = channel::unbounded_default::<MatcherState>();
    let settings = search_config.settings;
    let stop = Arc::new(AtomicBool::new(false));

    if let Some(scope) = search_config.git_search_scope.clone() {
        let _ = spawn_git_searcher(scope, stop.clone(), tx_items);
    } else if let Some(items) = stdin_items {
        let _ = spawn_stdin_searcher(items, stop.clone(), tx_items);
    } else {
        let _ = spawn_searcher_with_config(search_config, stop.clone(), tx_items);
    }

    if settings.matcher.is_exact() {
        let _ = spawn_exact_matcher(
            rx_items,
            rx_cmd,
            stop,
            tx_state,
            settings.mode.is_file_name_only(),
            settings.mode.is_content(),
            query.clone(),
        );
    } else {
        let _ = spawn_matcher(
            rx_items,
            rx_cmd,
            stop,
            tx_state,
            settings.mode.is_file_name_only(),
            settings.mode.is_content(),
        );
    }
    let _ = tx_cmd.send(MatcherCommand::Query(query));

    loop {
        match rx_state.recv() {
            Ok(state) if !state.working => break,
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }

    while let Ok(Some(_)) = rx_state.try_recv() {}

    let _ = tx_cmd.send(MatcherCommand::Resize(u32::MAX));
    if let Ok(state) = rx_state.recv() {
        write_match_results(&state.results, settings.mode.is_content())?;
    }

    Ok(())
}

fn write_match_results(results: &[SearchResult], is_content_mode: bool) -> anyhow::Result<()> {
    let use_color = io::stdout().is_terminal();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for result in results {
        let line = if use_color && !result.indices.is_empty() {
            format_colored_result(
                &result.item,
                &result.indices,
                result.column,
                is_content_mode,
            )
        } else {
            format_item_output(&result.item, result.column, false)
        };
        writeln!(out, "{line}")?;
    }

    Ok(())
}

fn colorize_chars(text: &str, indices: &[u32]) -> String {
    if indices.is_empty() {
        return text.to_string();
    }

    let mut sorted: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
    sorted.sort_unstable();
    sorted.dedup();

    let mut result = String::with_capacity(text.len() + sorted.len() * 9);
    let mut match_pos = 0;
    let mut in_match = false;

    for (char_idx, ch) in text.chars().enumerate() {
        let is_match = match_pos < sorted.len() && sorted[match_pos] == char_idx;
        if is_match {
            match_pos += 1;
        }
        if is_match && !in_match {
            result.push_str("\x1b[36m");
            in_match = true;
        } else if !is_match && in_match {
            result.push_str("\x1b[0m");
            in_match = false;
        }
        result.push(ch);
    }

    if in_match {
        result.push_str("\x1b[0m");
    }

    result
}

fn format_colored_result(
    item: &SearchItem,
    indices: &[u32],
    column: Option<usize>,
    is_content_mode: bool,
) -> String {
    match item {
        SearchItem::Stdin(text) | SearchItem::Message(text) => colorize_chars(text, indices),
        SearchItem::Path(path) => colorize_chars(path, indices),
        SearchItem::Grep { path, line, text } if is_content_mode => {
            let line_num = line.to_string();
            let content = text.trim_end_matches(['\n', '\r']);
            let path_char_len = path.chars().count();
            let prefix_char_len = path_char_len + 1 + line_num.chars().count() + 1;
            let path_indices: Vec<u32> = indices
                .iter()
                .filter(|&&i| (i as usize) < path_char_len)
                .copied()
                .collect();
            let content_indices: Vec<u32> = indices
                .iter()
                .filter_map(|&i| {
                    let i = i as usize;
                    if i >= prefix_char_len {
                        Some((i - prefix_char_len) as u32)
                    } else {
                        None
                    }
                })
                .collect();

            let colored_path = colorize_chars(path, &path_indices);
            let colored_content = colorize_chars(content, &content_indices);

            if let Some(col) = column {
                format!(
                    "{}:{}:\x1b[2m{}\x1b[0m:{}",
                    colored_path, line_num, col, colored_content
                )
            } else {
                format!("{}:{}:{}", colored_path, line_num, colored_content)
            }
        }
        SearchItem::GitHistory { .. }
        | SearchItem::GitBranch { .. }
        | SearchItem::GitCommit { .. } => format_item_output(item, column, false),
        SearchItem::Grep { .. } => format_item_output(item, column, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorize_chars_groups_consecutive_matches() {
        assert_eq!(
            colorize_chars("abcd", &[1, 2]),
            "a\x1b[36mbc\x1b[0md".to_string()
        );
    }

    #[test]
    fn format_colored_result_offsets_grep_content_indices() {
        let item = SearchItem::grep("src/main.rs", 7, "hello world");
        let rendered = format_colored_result(&item, &[0, 1, 14, 15, 16], None, true);

        assert!(rendered.contains("\x1b[36msr\x1b[0mc/main.rs"));
        assert!(rendered.contains("\x1b[36mhel\x1b[0mlo world"));
    }
}
