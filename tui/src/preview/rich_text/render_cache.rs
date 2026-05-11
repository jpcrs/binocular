use std::borrow::Cow;
use std::path::Path;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};

use crate::preview::rich_text::syntax::{
    detect_language, get_configs, get_highlighter, get_style, get_syntax_set, get_theme_set,
    SyntaxRegistry,
};
use crate::preview::rich_text::{RichTextDocument, TextBuffer};

pub fn create_rich_text_document(content: String, path: &Path) -> RichTextDocument {
    let buffer = TextBuffer::new(content);
    let raw_lines = buffer.line_ranges().to_vec();
    let lines = build_display_lines(buffer.as_str(), &raw_lines, path);
    let tree = build_syntax_tree(buffer.as_str(), path);

    RichTextDocument {
        buffer,
        lines,
        tree,
        dirty: false,
    }
}

const MAX_LINE_BYTES_FOR_HIGHLIGHTING: usize = 8_000;

fn build_display_lines(
    content: &str,
    raw_lines: &[(usize, usize)],
    path: &Path,
) -> Vec<Line<'static>> {
    let has_huge_line = raw_lines
        .iter()
        .any(|(s, e)| e - s > MAX_LINE_BYTES_FOR_HIGHLIGHTING);
    if !has_huge_line {
        if let Some(config) =
            detect_language(path).and_then(|lang| get_configs().get(lang).cloned())
        {
            if let Some(lines) = try_build_highlighted_lines(content, raw_lines, &config) {
                return lines;
            }
        }

        if let Some(lines) = try_build_syntect_lines(content, raw_lines, path) {
            return lines;
        }
    }

    build_plain_lines(content, raw_lines)
}

fn try_build_highlighted_lines(
    content: &str,
    raw_lines: &[(usize, usize)],
    config: &std::sync::Arc<HighlightConfiguration>,
) -> Option<Vec<Line<'static>>> {
    let mut highlighter = get_highlighter().write().ok()?;
    let events = highlighter
        .highlight(config, content.as_bytes(), None, |_| None)
        .ok()?;
    let mut builder = HighlightedLineBuilder::new();

    for event in events {
        match event {
            Ok(HighlightEvent::Source { start, end }) => builder.push_source(&content[start..end]),
            Ok(HighlightEvent::HighlightStart(highlight)) => {
                builder.push_style(get_style(highlight.0))
            }
            Ok(HighlightEvent::HighlightEnd) => builder.pop_style(),
            Err(_) => return None,
        }
    }

    let mut lines = builder.finish();
    while lines.len() < raw_lines.len() {
        lines.push(Line::from(vec![line_number_span(lines.len() + 1)]));
    }
    Some(lines)
}

fn try_build_syntect_lines(
    content: &str,
    raw_lines: &[(usize, usize)],
    path: &Path,
) -> Option<Vec<Line<'static>>> {
    let ss = get_syntax_set();
    let ts = get_theme_set();
    let ext = path.extension()?.to_str()?;
    let syntax = ss.find_syntax_by_extension(ext)?;
    let theme = ts.themes.get("base16-ocean.dark")?;
    let mut h = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for (line_idx, line) in LinesWithEndings::from(content).enumerate() {
        let ranges = h.highlight_line(line, ss).ok()?;
        let mut spans = vec![line_number_span(line_idx + 1)];
        for (style, text) in &ranges {
            let text = text.trim_end_matches('\n').trim_end_matches('\r');
            if text.is_empty() {
                continue;
            }
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(
                sanitize_content(text).into_owned(),
                Style::default().fg(fg),
            ));
        }
        lines.push(Line::from(spans));
    }

    while lines.len() < raw_lines.len() {
        lines.push(Line::from(vec![line_number_span(lines.len() + 1)]));
    }

    Some(lines)
}

fn build_plain_lines(content: &str, raw_lines: &[(usize, usize)]) -> Vec<Line<'static>> {
    raw_lines
        .iter()
        .enumerate()
        .map(|(idx, (start, end))| plain_line(content, idx + 1, *start, *end))
        .collect()
}

fn plain_line(content: &str, line_number: usize, start: usize, end: usize) -> Line<'static> {
    let line_content = &content[start..end];
    let trimmed = line_content.trim_end_matches('\n').trim_end_matches('\r');
    let sanitized = sanitize_content(trimmed);
    Line::from(vec![
        line_number_span(line_number),
        Span::raw(sanitized.into_owned()),
    ])
}

fn line_number_span(line_number: usize) -> Span<'static> {
    Span::styled(
        format!("{:4} ", line_number),
        Style::default().fg(Color::DarkGray),
    )
}

fn build_syntax_tree(content: &str, path: &Path) -> Option<tree_sitter::Tree> {
    let lang = detect_language(path)?;
    let language = SyntaxRegistry::instance().get_language(lang)?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(content, None)
}

struct HighlightedLineBuilder {
    lines: Vec<Line<'static>>,
    current_line_spans: Vec<Span<'static>>,
    line_idx: usize,
    style_stack: Vec<Style>,
}

impl HighlightedLineBuilder {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_line_spans: vec![line_number_span(1)],
            line_idx: 1,
            style_stack: Vec::new(),
        }
    }

    fn push_source(&mut self, text: &str) {
        for (idx, part) in text.split('\n').enumerate() {
            if idx > 0 {
                self.push_line_break();
            }
            self.push_text_segment(part);
        }
    }

    fn push_text_segment(&mut self, segment: &str) {
        let trimmed = segment.trim_end_matches('\r');
        if trimmed.is_empty() {
            return;
        }

        let style = self.current_style();
        self.current_line_spans
            .push(Span::styled(sanitize_content(trimmed).into_owned(), style));
    }

    fn push_line_break(&mut self) {
        self.lines
            .push(Line::from(std::mem::take(&mut self.current_line_spans)));
        self.line_idx += 1;
        self.current_line_spans
            .push(line_number_span(self.line_idx));
    }

    fn push_style(&mut self, style: Style) {
        self.style_stack.push(style);
    }

    fn pop_style(&mut self) {
        let _ = self.style_stack.pop();
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if !self.current_line_spans.is_empty() {
            self.lines.push(Line::from(self.current_line_spans));
        }
        self.lines
    }
}

pub fn regenerate_lines(text_file: &mut RichTextDocument, path: &Path) {
    if !text_file.dirty {
        return;
    }

    let raw_lines = text_file.raw_lines().to_vec();
    text_file.lines = build_display_lines(text_file.content(), &raw_lines, path);
    text_file.tree = build_syntax_tree(text_file.content(), path);
    text_file.dirty = false;
}

pub fn generate_plain_lines_for_range(
    text_file: &RichTextDocument,
    start_line: usize,
    end_line: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(end_line - start_line);

    for i in start_line..end_line {
        if let Some((start, end)) = text_file.line_range(i) {
            lines.push(plain_line(text_file.content(), i + 1, start, end));
        }
    }

    lines
}

pub fn sanitize_content(content: &str) -> Cow<'_, str> {
    if content.contains('\t') {
        Cow::Owned(content.replace('\t', "    "))
    } else {
        Cow::Borrowed(content)
    }
}
