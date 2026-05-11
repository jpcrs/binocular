//! Archive file preview: lists entries from ZIP, tar.gz/bz2/xz archives.

mod detect;
mod listing;
mod tar;
mod zip;

use ratatui::text::Text;
use std::path::Path;

pub use detect::ArchiveKind;
pub fn detect_archive_kind(path: &Path) -> Option<ArchiveKind> {
    detect::detect_archive_kind(path)
}

pub fn generate_preview(path: &Path, kind: ArchiveKind) -> Text<'static> {
    match kind {
        ArchiveKind::Zip => zip::preview_zip(path),
        ArchiveKind::TarGz => tar::preview_tar_gz(path),
        ArchiveKind::TarBz2 => tar::preview_tar_bz2(path),
        ArchiveKind::TarXz => tar::preview_tar_xz(path),
        ArchiveKind::TarRaw => tar::preview_tar_raw(path),
    }
}
