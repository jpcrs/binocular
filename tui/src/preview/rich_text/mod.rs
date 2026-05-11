mod buffer;
pub mod commands;
mod edit;
pub mod edit_ops;
pub mod motion;
mod render_cache;
pub mod search;
pub mod syntax;
pub mod text_objects;
mod types;
pub(crate) mod ui;
pub mod undo;
pub mod utils;
pub mod yank;

pub use buffer::{TextBuffer, TextEdit, TextEditKind, TextUndoFrame};
pub use edit::{
    apply_text_edit, edit_content_delete_char, edit_content_delete_char_at,
    edit_content_delete_range, edit_content_insert_char, edit_content_insert_text,
};
pub use render_cache::{
    create_rich_text_document, generate_plain_lines_for_range, regenerate_lines, sanitize_content,
};
pub use types::RichTextDocument;
