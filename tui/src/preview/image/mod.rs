mod detect;
mod metadata;
pub(crate) mod ui;

use image::ImageReader;
use ratatui::text::Text;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::path::Path;

pub fn is_image_extension(path: &Path) -> bool {
    detect::is_image_extension(path)
}

pub fn load_image(path: &Path, picker: &Picker) -> Option<(StatefulProtocol, Text<'static>)> {
    let reader = ImageReader::open(path).ok()?;
    let reader = reader.with_guessed_format().ok()?;
    let format = reader.format();
    let dyn_img = reader.decode().ok()?;
    let metadata = metadata::generate_metadata(path, &dyn_img, format);
    let protocol = picker.new_resize_protocol(dyn_img);
    Some((protocol, metadata))
}
