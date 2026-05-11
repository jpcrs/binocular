use crate::preview::rich_text::RichTextDocument;
use crate::preview::structured_log::{LogFilterState, StructuredLog};
use ratatui::text::Text;
use ratatui_image::protocol::StatefulProtocol;

pub struct DiffPreview {
    pub text: Text<'static>,
}

pub struct ImagePreview {
    pub metadata: Text<'static>,
    pub metadata_line_count: usize,
    pub protocol: StatefulProtocol,
}

pub struct MediaPreview {
    pub metadata: Text<'static>,
    pub metadata_line_count: usize,
    pub artwork: Option<StatefulProtocol>,
}

pub struct LogPreview {
    pub log: StructuredLog,
    pub filter_state: LogFilterState,
}

pub enum PreviewContent {
    RichText(RichTextDocument),
    Diff(DiffPreview),
    PlainText(Text<'static>),
    Image(ImagePreview),
    Media(MediaPreview),
    StructuredLog(LogPreview),
}
