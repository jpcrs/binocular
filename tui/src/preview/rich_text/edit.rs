use crate::preview::rich_text::{RichTextDocument, TextEdit};

pub fn apply_text_edit(text_file: &mut RichTextDocument, edit: &TextEdit) -> bool {
    let applied = text_file.buffer.apply_edit(edit);
    if applied {
        text_file.invalidate_caches();
    }
    applied
}

pub fn edit_content_insert_char(
    text_file: &mut RichTextDocument,
    byte_idx: usize,
    c: char,
) -> Option<TextEdit> {
    let edit = text_file.buffer.insert_char(byte_idx, c)?;
    text_file.invalidate_caches();
    Some(edit)
}

pub fn edit_content_insert_text(
    text_file: &mut RichTextDocument,
    byte_idx: usize,
    text: String,
) -> Option<TextEdit> {
    let edit = text_file.buffer.insert_text(byte_idx, text)?;
    text_file.invalidate_caches();
    Some(edit)
}

pub fn edit_content_delete_char(
    text_file: &mut RichTextDocument,
    byte_idx: usize,
) -> Option<TextEdit> {
    let edit = text_file.buffer.delete_char_before(byte_idx)?;
    text_file.invalidate_caches();
    Some(edit)
}

pub fn edit_content_delete_char_at(
    text_file: &mut RichTextDocument,
    byte_idx: usize,
) -> Option<TextEdit> {
    let edit = text_file.buffer.delete_char_at(byte_idx)?;
    text_file.invalidate_caches();
    Some(edit)
}

pub fn edit_content_delete_range(
    text_file: &mut RichTextDocument,
    start: usize,
    end: usize,
) -> Option<TextEdit> {
    let deleted = text_file.buffer.delete_range(start..end)?;
    text_file.invalidate_caches();
    Some(TextEdit::delete(start, deleted))
}
