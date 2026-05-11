//! Archive format detection.

use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub enum ArchiveKind {
    Zip,
    TarGz,
    TarBz2,
    TarXz,
    TarRaw,
}

pub fn detect_archive_kind(path: &Path) -> Option<ArchiveKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();

    if name.ends_with(".zip")
        || name.ends_with(".jar")
        || name.ends_with(".war")
        || name.ends_with(".ear")
        || name.ends_with(".apk")
        || name.ends_with(".ipa")
        || name.ends_with(".whl")
        || name.ends_with(".xlsx")
        || name.ends_with(".docx")
        || name.ends_with(".pptx")
        || name.ends_with(".odt")
        || name.ends_with(".ods")
    {
        return Some(ArchiveKind::Zip);
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ArchiveKind::TarGz);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
        return Some(ArchiveKind::TarBz2);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Some(ArchiveKind::TarXz);
    }
    if name.ends_with(".tar") {
        return Some(ArchiveKind::TarRaw);
    }
    None
}
