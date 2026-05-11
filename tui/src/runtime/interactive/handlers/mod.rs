pub mod help;
pub mod key;
pub mod preview;
pub mod search;

pub use help::handle_help_modal_input;
pub use key::handle_input;
pub use preview::{
    handle_preview_mode_input, scroll_preview_page, sync_preview, toggle_window_mode,
};
pub use search::handle_search_mode_input;

pub(crate) fn check_infinite_scroll(
    app: &crate::app::App,
    item_limit: &mut u32,
    tx_cmd: &impl crate::infra::channel::Sender<crate::search::matcher::MatcherCommand>,
) {
    if app.search_session.search.total_matches > *item_limit as u64
        && app.search_session.search.selection
            >= app.search_session.search.results.len().saturating_sub(10)
    {
        *item_limit += 100;
        let _ = tx_cmd.send(crate::search::matcher::MatcherCommand::Resize(*item_limit));
    }
}
