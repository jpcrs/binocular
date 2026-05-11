pub const EMPTY_STRING: &str = "";
pub const TAB_WIDTH: usize = 4;

pub fn truncate_str_chars(s: &str, max_chars: usize) -> (&str, bool) {
    if s.len() <= max_chars {
        return (s, false);
    }
    match s.char_indices().nth(max_chars) {
        Some((byte_pos, _)) => (&s[..byte_pos], true),
        None => (s, false),
    }
}

const NULL_SYMBOL: char = '\u{2400}';
const TAB_CHARACTER: char = '\t';
const LINE_FEED_CHARACTER: char = '\x0A';
const CARRIAGE_RETURN_CHARACTER: char = '\r';
const DELETE_CHARACTER: char = '\x7F';
const BOM_CHARACTER: char = '\u{FEFF}';
const NULL_CHARACTER: char = '\x00';
const UNIT_SEPARATOR_CHARACTER: char = '\u{001F}';
const APPLICATION_PROGRAM_COMMAND_CHARACTER: char = '\u{009F}';

pub struct ReplaceNonPrintableConfig {
    pub replace_tab: bool,
    pub tab_width: usize,
    pub replace_line_feed: bool,
    pub replace_control_characters: bool,
}

impl ReplaceNonPrintableConfig {
    pub fn tab_width(&mut self, tab_width: usize) -> &mut Self {
        self.tab_width = tab_width;
        self
    }

    pub fn keep_line_feed(&mut self) -> &mut Self {
        self.replace_line_feed = false;
        self
    }

    pub fn keep_control_characters(&mut self) -> &mut Self {
        self.replace_control_characters = false;
        self
    }
}

impl Default for ReplaceNonPrintableConfig {
    fn default() -> Self {
        Self {
            replace_tab: true,
            tab_width: TAB_WIDTH,
            replace_line_feed: true,
            replace_control_characters: true,
        }
    }
}

pub fn next_char_boundary(s: &str, start: usize) -> usize {
    let mut i = start;
    let len = s.len();
    if i >= len {
        return len;
    }
    while !s.is_char_boundary(i) && i < len {
        i += 1;
    }
    i
}

pub fn prev_char_boundary(s: &str, start: usize) -> usize {
    let mut i = start;
    while !s.is_char_boundary(i) && i > 0 {
        i -= 1;
    }
    i
}

pub fn slice_at_char_boundaries(s: &str, start_byte_index: usize, end_byte_index: usize) -> &str {
    if start_byte_index > end_byte_index || start_byte_index > s.len() || end_byte_index > s.len() {
        return EMPTY_STRING;
    }
    &s[prev_char_boundary(s, start_byte_index)..next_char_boundary(s, end_byte_index)]
}

pub fn slice_up_to_char_boundary(s: &str, byte_index: usize) -> &str {
    &s[..next_char_boundary(s, byte_index)]
}

pub fn try_parse_utf8_char(input: &[u8]) -> Option<(char, usize)> {
    let str_from_utf8 = |seq| std::str::from_utf8(seq).ok();

    let decoded = input
        .get(0..1)
        .and_then(str_from_utf8)
        .map(|c| (c, 1))
        .or_else(|| input.get(0..2).and_then(str_from_utf8).map(|c| (c, 2)))
        .or_else(|| input.get(0..3).and_then(str_from_utf8).map(|c| (c, 3)))
        .or_else(|| input.get(0..4).and_then(str_from_utf8).map(|c| (c, 4)));

    decoded.map(|(seq, n)| (seq.chars().next().unwrap(), n))
}

pub fn replace_non_printable(
    input: &[u8],
    config: &ReplaceNonPrintableConfig,
) -> (String, Vec<i16>) {
    let mut output = String::with_capacity(input.len());
    let mut offsets = Vec::new();
    let mut cumulative_offset: i16 = 0;

    let mut idx = 0;
    let len = input.len();
    while idx < len {
        if let Some((chr, skip_ahead)) = try_parse_utf8_char(&input[idx..]) {
            for _ in 0..skip_ahead {
                offsets.push(cumulative_offset);
            }
            idx += skip_ahead;
            match chr {
                TAB_CHARACTER if config.replace_tab => {
                    output.push_str(&" ".repeat(config.tab_width));
                    cumulative_offset += i16::try_from(config.tab_width).unwrap() - 1;
                }
                LINE_FEED_CHARACTER => {
                    if config.replace_line_feed {
                        cumulative_offset -= 1;
                    } else {
                        output.push(chr);
                    }
                }
                CARRIAGE_RETURN_CHARACTER => {
                    cumulative_offset -= 1;
                }
                NULL_CHARACTER..=UNIT_SEPARATOR_CHARACTER
                | DELETE_CHARACTER..=APPLICATION_PROGRAM_COMMAND_CHARACTER
                | BOM_CHARACTER
                    if config.replace_control_characters =>
                {
                    output.push(NULL_SYMBOL);
                }
                // Unicode characters above 0x0700 seem unstable with ratatui
                c if c > '\u{0700}' => {
                    output.push(NULL_SYMBOL);
                }
                c => output.push(c),
            }
        } else {
            offsets.push(cumulative_offset);
            output.push(NULL_SYMBOL);
            idx += 1;
        }
    }

    (output, offsets)
}

const MAX_LINE_LENGTH: usize = 300;

pub fn preprocess_line(line: &str) -> (String, Vec<i16>) {
    replace_non_printable(
        {
            if line.len() > MAX_LINE_LENGTH {
                slice_up_to_char_boundary(line, MAX_LINE_LENGTH)
            } else {
                line
            }
        }
        .as_bytes(),
        &ReplaceNonPrintableConfig::default(),
    )
}

pub fn sanitize_text_with_indices(display_str: &str, indices: &[u32]) -> (String, Vec<u32>) {
    if display_str.is_ascii()
        && !display_str
            .bytes()
            .any(|b| b == b'\t' || b == b'\n' || b < 32 || b == 127)
    {
        return (display_str.to_string(), indices.to_vec());
    }

    let (printable, transformation_offsets) = preprocess_line(display_str);
    let mut match_indices = Vec::with_capacity(indices.len());

    for &start in indices {
        if start < u32::try_from(transformation_offsets.len()).unwrap() {
            let new_start = i64::from(start) + i64::from(transformation_offsets[start as usize]);
            match_indices.push(u32::try_from(new_start).unwrap_or(0));
        }
    }

    (printable, match_indices)
}

pub fn shrink_with_ellipsis(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        return s.to_string();
    }

    let half_max_length = (max_length / 2).saturating_sub(2);
    let first_half = slice_up_to_char_boundary(s, half_max_length);
    let second_half = slice_at_char_boundaries(s, s.len() - half_max_length, s.len());
    format!("{first_half}…{second_half}")
}

pub const PRINTABLE_ASCII_THRESHOLD: f32 = 0.7;

pub fn proportion_of_printable_ascii_characters(buffer: &[u8]) -> f32 {
    if buffer.is_empty() {
        return 1.0;
    }
    let mut printable: usize = 0;
    for &byte in buffer {
        if (32..127).contains(&byte) || byte == 9 || byte == 10 || byte == 13 {
            printable += 1;
        }
    }
    printable as f32 / buffer.len() as f32
}

pub fn find_first_match_column_in_grep_result(text: &str, match_indices: &[u32]) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut first_colon = None;

    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            if let Some(start) = first_colon {
                if i > start + 1 {
                    let potential_line_num = &text[start + 1..i];
                    if potential_line_num.bytes().all(|b| b.is_ascii_digit()) {
                        let content_start_byte = i + 1;
                        let content_start_char = if text.is_ascii() {
                            content_start_byte
                        } else {
                            text[..content_start_byte].chars().count()
                        };

                        if let Some(&first_match_idx) = match_indices
                            .iter()
                            .find(|&&idx| idx as usize >= content_start_char)
                        {
                            let column_offset = (first_match_idx as usize) - content_start_char;
                            return Some(column_offset + 1); // 1-indexed
                        }
                        return None;
                    }
                }

                first_colon = Some(i);
            } else {
                first_colon = Some(i);
            }
        }
    }
    None
}
