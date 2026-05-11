use super::fallback::build_text_or_binary_preview;
use crate::preview::structured_log;
use crate::preview::types::PreviewContent;
use crate::preview::{archive, directory, image, media, pdf, sqlite, types};
use ratatui::text::Text;
use ratatui_image::picker::Picker;
use std::path::Path;

pub(crate) struct PreviewBuildContext<'a> {
    pub picker: &'a Picker,
    pub log_max_entries: usize,
}

pub(crate) trait PathPreviewer: Sync {
    fn try_build(&self, path: &Path, ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent>;
}

struct DirectoryPreviewer;
struct ImagePreviewer;
struct MediaPreviewer;
struct ArchivePreviewer;
struct SqlitePreviewer;
struct PdfPreviewer;
struct StructuredLogPreviewer;
struct TextOrBinaryPreviewer;

static DIRECTORY_PREVIEWER: DirectoryPreviewer = DirectoryPreviewer;
static IMAGE_PREVIEWER: ImagePreviewer = ImagePreviewer;
static MEDIA_PREVIEWER: MediaPreviewer = MediaPreviewer;
static ARCHIVE_PREVIEWER: ArchivePreviewer = ArchivePreviewer;
static SQLITE_PREVIEWER: SqlitePreviewer = SqlitePreviewer;
static PDF_PREVIEWER: PdfPreviewer = PdfPreviewer;
static STRUCTURED_LOG_PREVIEWER: StructuredLogPreviewer = StructuredLogPreviewer;
static TEXT_OR_BINARY_PREVIEWER: TextOrBinaryPreviewer = TextOrBinaryPreviewer;

static PREVIEWERS: [&dyn PathPreviewer; 8] = [
    &DIRECTORY_PREVIEWER,
    &IMAGE_PREVIEWER,
    &MEDIA_PREVIEWER,
    &ARCHIVE_PREVIEWER,
    &SQLITE_PREVIEWER,
    &PDF_PREVIEWER,
    &STRUCTURED_LOG_PREVIEWER,
    &TEXT_OR_BINARY_PREVIEWER,
];

pub(crate) fn build_path_preview(
    path_str: &str,
    picker: &Picker,
    log_max_entries: usize,
) -> PreviewContent {
    let path = Path::new(path_str);
    if !path.exists() {
        return PreviewContent::PlainText(Text::default());
    }

    let ctx = PreviewBuildContext {
        picker,
        log_max_entries,
    };
    for previewer in PREVIEWERS {
        if let Some(preview) = previewer.try_build(path, &ctx) {
            return preview;
        }
    }

    PreviewContent::PlainText(Text::default())
}

impl PathPreviewer for DirectoryPreviewer {
    fn try_build(&self, path: &Path, _ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        path.is_dir()
            .then(|| PreviewContent::PlainText(directory::generate_preview(path)))
    }
}

impl PathPreviewer for ImagePreviewer {
    fn try_build(&self, path: &Path, ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        if !path.is_file() || !image::is_image_extension(path) {
            return None;
        }

        let (protocol, metadata) = image::load_image(path, ctx.picker)?;
        let metadata_line_count = metadata.lines.len();
        Some(PreviewContent::Image(types::ImagePreview {
            protocol,
            metadata,
            metadata_line_count,
        }))
    }
}

impl PathPreviewer for MediaPreviewer {
    fn try_build(&self, path: &Path, ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        let kind = media::detect_media_kind(path)?;
        let preview = media::generate_preview(path, kind);
        let metadata_line_count = preview.text.lines.len();
        let artwork = preview
            .artwork_bytes
            .as_deref()
            .and_then(|bytes| ::image::load_from_memory(bytes).ok())
            .map(|img| ctx.picker.new_resize_protocol(img));

        Some(PreviewContent::Media(types::MediaPreview {
            metadata: preview.text,
            metadata_line_count,
            artwork,
        }))
    }
}

impl PathPreviewer for ArchivePreviewer {
    fn try_build(&self, path: &Path, _ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        archive::detect_archive_kind(path)
            .map(|kind| PreviewContent::PlainText(archive::generate_preview(path, kind)))
    }
}

impl PathPreviewer for SqlitePreviewer {
    fn try_build(&self, path: &Path, _ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        sqlite::is_sqlite(path).then(|| PreviewContent::PlainText(sqlite::generate_preview(path)))
    }
}

impl PathPreviewer for PdfPreviewer {
    fn try_build(&self, path: &Path, _ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        pdf::is_pdf(path).then(|| PreviewContent::PlainText(pdf::generate_preview(path)))
    }
}

impl PathPreviewer for StructuredLogPreviewer {
    fn try_build(&self, path: &Path, ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        let format = structured_log::detect_structured_log(path)?;
        let (entries, total_lines, all_fields) =
            structured_log::parse_initial(path, &format, ctx.log_max_entries);
        Some(structured_log::preview_content(
            structured_log::StructuredLog {
                entries,
                total_lines,
                all_fields,
                format,
            },
        ))
    }
}

impl PathPreviewer for TextOrBinaryPreviewer {
    fn try_build(&self, path: &Path, _ctx: &PreviewBuildContext<'_>) -> Option<PreviewContent> {
        if !path.is_file() {
            return None;
        }

        Some(build_text_or_binary_preview(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str, ext: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("binocular-preview-{name}-{nanos}.{ext}"))
    }

    #[test]
    fn structured_log_preview_wins_before_text_fallback() {
        let path = unique_temp_path("structured", "log");
        std::fs::write(&path, "{\"level\":\"info\",\"msg\":\"hello\"}\n").unwrap();

        let preview =
            build_path_preview(&path.display().to_string(), &Picker::halfblocks(), 10_000);

        assert!(matches!(preview, PreviewContent::StructuredLog(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn text_fallback_handles_plain_text_files() {
        let path = unique_temp_path("text", "txt");
        std::fs::write(&path, "hello world\n").unwrap();

        let preview =
            build_path_preview(&path.display().to_string(), &Picker::halfblocks(), 10_000);

        assert!(matches!(preview, PreviewContent::RichText(_)));
        let _ = std::fs::remove_file(path);
    }
}
