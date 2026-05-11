//! Binary entropy analysis.

use crate::preview::doc::PreviewDoc;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::fs;
use std::io::{Seek, SeekFrom};

pub fn append_analysis(file: &mut fs::File, doc: &mut PreviewDoc) {
    let file_size = file
        .metadata()
        .map(|metadata| metadata.len() as usize)
        .unwrap_or_default();

    let _ = file.seek(SeekFrom::Start(0));
    let sample = super::read_prefix(file, super::ENTROPY_SAMPLE_BYTES.min(file_size));

    doc.push_section(super::SECTION_ANALYSIS);

    let entropy = calculate_entropy(&sample);
    let sample_desc = if sample.len() < file_size {
        format!(" (sampled {} KB)", sample.len() / 1024)
    } else {
        String::new()
    };

    push_entropy(doc, entropy, sample_desc);
    push_entropy_label(doc, entropy_description(entropy));
    doc.push_blank_line();
}

pub fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut freq = [0u64; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

fn push_entropy(doc: &mut PreviewDoc, entropy: f64, sample_desc: String) {
    doc.push_line(Line::from(vec![
        Span::styled("   Entropy: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.2} bits/byte{}", entropy, sample_desc),
            Style::default().fg(Color::White),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            create_entropy_bar(entropy),
            Style::default().fg(entropy_color(entropy)),
        ),
    ]));
}

fn push_entropy_label(doc: &mut PreviewDoc, label: &'static str) {
    doc.push_line(Line::from(vec![
        Span::styled("   ", Style::default()),
        Span::styled(
            label,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]));
}

fn create_entropy_bar(entropy: f64) -> String {
    let max_entropy = 8.0;
    let filled = ((entropy / max_entropy) * 20.0).round() as usize;
    let empty = 20 - filled.min(20);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn entropy_color(entropy: f64) -> Color {
    if entropy > 7.5 {
        Color::Red
    } else if entropy > 6.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn entropy_description(entropy: f64) -> &'static str {
    if entropy > 7.5 {
        "High"
    } else if entropy > 6.0 {
        "Medium"
    } else {
        "Low"
    }
}
