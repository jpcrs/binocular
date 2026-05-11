//! Hex dump rendering.

use crate::preview::doc::PreviewDoc;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub fn append_hex_dump(header: &[u8], doc: &mut PreviewDoc) {
    doc.push_section(super::SECTION_HEX_DUMP);
    for line in create_hex_dump(header, super::HEX_DUMP_BYTES) {
        doc.push_line(line);
    }
    doc.push_blank_line();
}

pub fn create_hex_dump(data: &[u8], max_bytes: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let bytes_to_show = data.len().min(max_bytes);

    for (i, chunk) in data[..bytes_to_show].chunks(16).enumerate() {
        let offset = format!("   {:08x}  ", i * 16);

        let hex: String = chunk
            .iter()
            .enumerate()
            .map(|(j, b)| {
                if j == 8 {
                    format!(" {:02x}", b)
                } else {
                    format!("{:02x} ", b)
                }
            })
            .collect();

        let padding = "   ".repeat(16 - chunk.len()) + if chunk.len() <= 8 { " " } else { "" };

        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        lines.push(Line::from(vec![
            Span::styled(offset, Style::default().fg(Color::DarkGray)),
            Span::styled(hex, Style::default().fg(Color::Cyan)),
            Span::styled(padding, Style::default()),
            Span::styled(" │", Style::default().fg(Color::DarkGray)),
            Span::styled(ascii, Style::default().fg(Color::Green)),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines
}
