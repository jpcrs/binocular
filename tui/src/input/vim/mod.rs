pub(crate) mod command_parser;
pub(crate) mod common_actions;
pub(crate) mod normal_actions;
pub(crate) mod operator_parser;
mod preview_controller;
mod query_controller;

pub(crate) mod commands {
    pub use crate::preview::rich_text::commands::*;
}

pub(crate) mod edit {
    pub use crate::preview::rich_text::edit_ops::*;
}

pub(crate) mod line_editor {
    pub use crate::editor::line_editor::*;
}

pub(crate) mod motion {
    pub use crate::preview::rich_text::motion::*;
}

pub(crate) mod normal_action_handler {
    pub use crate::input::vim::normal_actions::*;
}

pub(crate) mod operator_handler {
    pub use crate::input::vim::operator_parser::*;
}

pub(crate) mod search {
    pub use crate::preview::rich_text::search::*;
}

pub(crate) mod text_objects {
    pub use crate::preview::rich_text::text_objects::*;
}

pub(crate) mod undo {
    pub use crate::preview::rich_text::undo::*;
}

pub(crate) mod utils {
    pub use crate::preview::rich_text::utils::*;
}

pub(crate) mod yank {
    pub use crate::preview::rich_text::yank::*;
}

pub use preview_controller::handle_input;
pub use query_controller::{handle_search_input, SearchInputResult};
