use crate::app::App;
use crate::preview::PreviewContent;

pub fn perform_search(app: &mut App, forward: bool) {
    let query = &app.preview_session.preview.state.search_query;
    if query.is_empty() {
        return;
    }

    if let Some(PreviewContent::RichText(text)) = &app.preview_session.preview.content {
        let mut matches = Vec::new();
        for (i, (start, end)) in text.raw_lines().iter().enumerate() {
            let line_text = &text.content()[*start..*end];

            for (idx, _) in line_text.match_indices(query) {
                let char_idx: usize = line_text[..idx]
                    .chars()
                    .map(|c| if c == '\t' { 4 } else { 1 })
                    .sum();
                matches.push((i, char_idx));
            }
        }

        if matches.is_empty() {
            return;
        }

        let current = (
            app.preview_session.preview.state.cursor_line,
            app.preview_session.preview.state.cursor_char,
        );

        if forward {
            if let Some(m) = matches.iter().find(|&&m| m > current) {
                app.preview_session.preview.state.cursor_line = m.0;
                app.preview_session.preview.state.cursor_char = m.1;
            } else {
                if let Some(m) = matches.first() {
                    app.preview_session.preview.state.cursor_line = m.0;
                    app.preview_session.preview.state.cursor_char = m.1;
                }
            }
        } else {
            if let Some(m) = matches.iter().rev().find(|&&m| m < current) {
                app.preview_session.preview.state.cursor_line = m.0;
                app.preview_session.preview.state.cursor_char = m.1;
            } else {
                if let Some(m) = matches.last() {
                    app.preview_session.preview.state.cursor_line = m.0;
                    app.preview_session.preview.state.cursor_char = m.1;
                }
            }
        }
    }
}
