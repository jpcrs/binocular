use crate::preview::doc::{format_file_size, format_unix_timestamp, PreviewDoc};
use image::DynamicImage;
use ratatui::style::Color;
use ratatui::text::Text;
use std::fs;
use std::path::Path;

pub fn generate_metadata(
    path: &Path,
    img: &DynamicImage,
    format: Option<image::ImageFormat>,
) -> Text<'static> {
    let mut doc = PreviewDoc::new();

    doc.push_section("File Info");
    if let Ok(meta) = fs::metadata(path) {
        doc.push_field("Size", format_file_size(meta.len()), Color::White);
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                doc.push_field(
                    "Modified",
                    format_unix_timestamp(duration.as_secs()),
                    Color::White,
                );
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            doc.push_field("Permissions", format!("{:o}", mode & 0o777), Color::White);
        }
    }
    doc.push_blank_line();

    doc.push_section("Image Info");

    let format_name = format
        .map(|f| format!("{:?}", f))
        .unwrap_or_else(|| "Unknown".to_string());
    doc.push_field("Format", format_name, Color::Green);
    doc.push_field(
        "Dimensions",
        format!("{}×{} px", img.width(), img.height()),
        Color::White,
    );
    doc.push_field("Color", color_type_description(img.color()), Color::White);
    doc.push_field(
        "Bit Depth",
        format!(
            "{} bits/channel",
            img.color().bits_per_pixel() / img.color().channel_count() as u16
        ),
        Color::White,
    );
    let megapixels = (img.width() as f64 * img.height() as f64) / 1_000_000.0;
    doc.push_field("Megapixels", format!("{:.2} MP", megapixels), Color::White);
    doc.push_blank_line();

    doc.into_text()
}

fn color_type_description(color: image::ColorType) -> &'static str {
    match color {
        image::ColorType::L8 => "Grayscale (8-bit)",
        image::ColorType::La8 => "Grayscale + Alpha (8-bit)",
        image::ColorType::Rgb8 => "RGB (8-bit)",
        image::ColorType::Rgba8 => "RGBA (8-bit)",
        image::ColorType::L16 => "Grayscale (16-bit)",
        image::ColorType::La16 => "Grayscale + Alpha (16-bit)",
        image::ColorType::Rgb16 => "RGB (16-bit)",
        image::ColorType::Rgba16 => "RGBA (16-bit)",
        image::ColorType::Rgb32F => "RGB (32-bit float)",
        image::ColorType::Rgba32F => "RGBA (32-bit float)",
        _ => "Unknown",
    }
}
