use crate::preview::doc::PreviewDoc;
use std::fs;
use std::path::Path;

pub fn append_printable_strings(path: &Path, doc: &mut PreviewDoc) {
    let Ok(mut file) = fs::File::open(path) else {
        return;
    };

    let sample = super::read_prefix(&mut file, super::STRINGS_SAMPLE_BYTES);
    let strings = extract_printable_strings(&sample, super::MIN_PRINTABLE_STRING_LEN);
    if strings.is_empty() {
        return;
    }

    doc.push_section(super::SECTION_PRINTABLE_STRINGS);

    for (index, s) in strings
        .iter()
        .take(super::MAX_PRINTABLE_STRINGS_SHOWN)
        .enumerate()
    {
        let text = truncate_for_preview(s, super::MAX_STRING_PREVIEW_LEN);
        doc.push_indexed(index + 1, text);
    }

    if strings.len() > super::MAX_PRINTABLE_STRINGS_SHOWN {
        doc.push_muted_italic(format!(
            "   ... and {} more strings",
            strings.len() - super::MAX_PRINTABLE_STRINGS_SHOWN
        ));
    }
}

pub fn extract_printable_strings(data: &[u8], min_length: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = String::new();

    for &byte in data {
        if byte.is_ascii_graphic() || byte == b' ' {
            current.push(byte as char);
        } else {
            if current.len() >= min_length {
                strings.push(std::mem::take(&mut current));
            }
            current.clear();
        }
    }

    if current.len() >= min_length {
        strings.push(current);
    }

    strings
}

fn truncate_for_preview(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}
