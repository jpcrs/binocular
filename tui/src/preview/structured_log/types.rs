#[derive(Clone, Debug)]
pub enum LogFormat {
    Jsonl,
    Logfmt,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub fields: Vec<(String, String)>,
    pub raw: String,
}

#[derive(Clone, Debug)]
pub struct StructuredLog {
    pub entries: Vec<LogEntry>,
    pub total_lines: usize,
    pub all_fields: Vec<String>,
    pub format: LogFormat,
}

#[derive(Clone, Debug)]
pub struct ColumnConfig {
    pub field: String,
    pub width: usize,
}

#[derive(Clone, Debug)]
pub struct ColModal {
    pub cursor: usize,
    pub checked: Vec<bool>,
    pub scroll: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LogFilterState {
    pub input: String,
    pub filters: Vec<LogFilter>,
    pub input_active: bool,
    pub cursor: usize,
    pub scroll: usize,
    pub visible_cols: Vec<ColumnConfig>,
    pub selected_col: usize,
    pub col_scroll: usize,
    pub col_modal: Option<ColModal>,
    pub marked: std::collections::HashSet<usize>,
    pub cached_matches: Vec<usize>,
    pub paused: bool,
}

#[derive(Clone, Debug)]
pub struct LogFilter {
    pub field: Option<String>,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterOp {
    Equals,
    Contains,
    NotEquals,
    Since(u64),
}
