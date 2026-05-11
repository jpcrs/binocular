use std::path::Path;

pub fn extract_page_text(pdf: &lopdf::Document, page_id: lopdf::ObjectId) -> lopdf::Result<String> {
    let content = pdf.get_and_decode_page_content(page_id)?;

    let mut text = String::new();
    let mut in_text_block = false;
    let mut last_was_newline = false;

    for op in &content.operations {
        match op.operator.as_str() {
            "BT" => {
                in_text_block = true;
            }
            "ET" => {
                in_text_block = false;
                if !text.ends_with('\n') {
                    text.push('\n');
                    last_was_newline = true;
                }
            }
            "Tj" | "TJ" if in_text_block => {
                for operand in &op.operands {
                    append_operand_text(operand, &mut text);
                }
                last_was_newline = false;
            }
            "'" | "\"" if in_text_block => {
                if !last_was_newline {
                    text.push('\n');
                }
                for operand in &op.operands {
                    append_operand_text(operand, &mut text);
                }
                last_was_newline = false;
            }
            "Td" | "TD" | "T*" if in_text_block => {
                if !last_was_newline && !text.is_empty() {
                    text.push('\n');
                    last_was_newline = true;
                }
            }
            _ => {}
        }
    }

    Ok(text)
}

pub fn decode_pdf_string(bytes: &[u8]) -> String {
    decode_pdf_bytes(bytes).trim().to_string()
}

pub fn extract_all_text(path: &Path) -> lopdf::Result<Vec<String>> {
    let pdf = lopdf::Document::load(path)?;
    let mut page_ids: Vec<(u32, lopdf::ObjectId)> = pdf.get_pages().into_iter().collect();
    page_ids.sort_by_key(|(n, _)| *n);

    let mut lines = Vec::new();
    for (_page_num, page_id) in &page_ids {
        let text = extract_page_text(&pdf, *page_id)?;
        for line in text.lines() {
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                lines.push(trimmed);
            }
        }
    }
    Ok(lines)
}

fn append_operand_text(operand: &lopdf::Object, out: &mut String) {
    match operand {
        lopdf::Object::String(bytes, _) => {
            out.push_str(&decode_pdf_bytes(bytes));
        }
        lopdf::Object::Array(items) => {
            for item in items {
                match item {
                    lopdf::Object::String(bytes, _) => {
                        out.push_str(&decode_pdf_bytes(bytes));
                    }
                    lopdf::Object::Integer(n) if *n < -100 => {
                        out.push(' ');
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn decode_pdf_bytes(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16).to_string();
    }

    bytes
        .iter()
        .map(|&b| if b >= 0x20 { b as char } else { ' ' })
        .collect()
}
