use crate::app::{App, AppAction};
use crate::preview::PreviewContent;

pub fn save_file(app: &mut App) {
    if let Some(path_str) = app.preview_file_path() {
        if let Some(PreviewContent::RichText(text_file)) = &app.preview_session.preview.content {
            let path = std::path::Path::new(path_str);
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    app.preview_session.preview.state.status_message = Some((
                        "Error: cannot save through a symlink".to_string(),
                        std::time::Instant::now(),
                    ));
                    return;
                }
            }
            if let Err(e) = std::fs::write(path_str, text_file.content()) {
                eprintln!("Error saving file: {}", e);
                app.preview_session.preview.state.status_message =
                    Some((format!("Error: {}", e), std::time::Instant::now()));
            } else {
                app.preview_session.preview.state.status_message =
                    Some(("File saved".to_string(), std::time::Instant::now()));
            }
        }
    }
}

pub fn execute_command(app: &mut App, cmd: &str) {
    if cmd == "w" {
        save_file(app);
    } else if cmd == "q" {
        app.ui.layout.preview_fullscreen = false;
        app.apply_action(AppAction::FocusSearch);
    } else if cmd == "wq" {
        save_file(app);
        app.apply_action(AppAction::Quit);
    }
}
