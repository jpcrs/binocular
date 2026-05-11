use std::path::Path;

pub fn is_image_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };

    extension.eq_ignore_ascii_case("jpg")
        || extension.eq_ignore_ascii_case("jpeg")
        || extension.eq_ignore_ascii_case("png")
        || extension.eq_ignore_ascii_case("gif")
        || extension.eq_ignore_ascii_case("webp")
        || extension.eq_ignore_ascii_case("bmp")
        || extension.eq_ignore_ascii_case("tiff")
        || extension.eq_ignore_ascii_case("ico")
        || extension.eq_ignore_ascii_case("svg")
}
