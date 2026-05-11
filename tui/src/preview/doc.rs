use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

pub struct PreviewDoc {
    lines: Vec<Line<'static>>,
}

impl PreviewDoc {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn push_line(&mut self, line: Line<'static>) {
        self.lines.push(line);
    }

    pub fn push_section(&mut self, title: &'static str) {
        self.lines.push(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    pub fn push_field<T: Into<String>>(&mut self, label: &str, value: T, value_color: Color) {
        self.lines.push(Line::from(vec![
            Span::styled(
                format!("   {}: ", label),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(value.into(), Style::default().fg(value_color)),
        ]));
    }

    pub fn push_indexed(&mut self, index: usize, text: String) {
        self.lines.push(Line::from(vec![
            Span::styled(
                format!("   {:2}: ", index),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(text, Style::default().fg(Color::White)),
        ]));
    }

    pub fn push_muted_italic<T: Into<String>>(&mut self, text: T) {
        self.lines.push(Line::from(vec![Span::styled(
            text.into(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    pub fn push_blank_line(&mut self) {
        self.lines.push(Line::from(""));
    }

    pub fn into_text(self) -> Text<'static> {
        Text::from(self.lines)
    }
}

pub fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB ({} bytes)", size as f64 / GB as f64, size)
    } else if size >= MB {
        format!("{:.2} MB ({} bytes)", size as f64 / MB as f64, size)
    } else if size >= KB {
        format!("{:.2} KB ({} bytes)", size as f64 / KB as f64, size)
    } else {
        format!("{} bytes", size)
    }
}

pub fn format_unix_timestamp(secs: u64) -> String {
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    let mut year = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in &days_in_months {
        if remaining_days < *days {
            break;
        }
        remaining_days -= *days;
        month += 1;
    }

    let day = remaining_days + 1;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hours, minutes
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
