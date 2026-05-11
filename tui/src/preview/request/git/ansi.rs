use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

pub(crate) fn parse_ansi_text(text: String) -> Text<'static> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut style = Style::default();
    let bytes = text.as_bytes();
    let mut idx = 0usize;

    while idx < bytes.len() {
        if bytes[idx] == 0x1b && idx + 1 < bytes.len() && bytes[idx + 1] == b'[' {
            if !current.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut current), style));
            }
            idx += 2;
            let start = idx;
            while idx < bytes.len() && bytes[idx] != b'm' {
                idx += 1;
            }
            if idx >= bytes.len() {
                break;
            }
            let codes = &text[start..idx];
            style = apply_ansi_codes(style, codes);
            idx += 1;
            continue;
        }

        let ch = text[idx..].chars().next().unwrap();
        idx += ch.len_utf8();
        if ch == '\n' {
            if !current.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut current), style));
            }
            lines.push(Line::from(std::mem::take(&mut spans)));
        } else if ch != '\r' {
            current.push(ch);
        }
    }

    if !current.is_empty() || !spans.is_empty() {
        spans.push(Span::styled(current, style));
        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

fn apply_ansi_codes(mut style: Style, codes: &str) -> Style {
    if codes.is_empty() {
        return Style::default();
    }

    for code in codes.split(';').filter(|part| !part.is_empty()) {
        match code {
            "0" => style = Style::default(),
            "1" => style = style.add_modifier(Modifier::BOLD),
            "2" => style = style.add_modifier(Modifier::DIM),
            "22" => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            "30" => style = style.fg(Color::Black),
            "31" => style = style.fg(Color::Red),
            "32" => style = style.fg(Color::Green),
            "33" => style = style.fg(Color::Yellow),
            "34" => style = style.fg(Color::Blue),
            "35" => style = style.fg(Color::Magenta),
            "36" => style = style.fg(Color::Cyan),
            "37" => style = style.fg(Color::Gray),
            "39" => style = style.fg(Color::Reset),
            "90" => style = style.fg(Color::DarkGray),
            "91" => style = style.fg(Color::LightRed),
            "92" => style = style.fg(Color::LightGreen),
            "93" => style = style.fg(Color::LightYellow),
            "94" => style = style.fg(Color::LightBlue),
            "95" => style = style.fg(Color::LightMagenta),
            "96" => style = style.fg(Color::LightCyan),
            "97" => style = style.fg(Color::White),
            _ => {}
        }
    }
    style
}
