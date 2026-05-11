use crate::preview::doc::format_unix_timestamp;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub fn generate_preview(path: &Path) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let abs_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    lines.push(Line::from(vec![Span::styled(
        format!("Directory: {}", abs_path.display()),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let entries = match collect_entries(path) {
        Ok(e) => e,
        Err(err) => {
            lines.push(Line::from(vec![Span::styled(
                format!("Error reading directory: {}", err),
                Style::default().fg(Color::Red),
            )]));
            return Text::from(lines);
        }
    };

    let dir_count = entries.iter().filter(|e| e.kind == EntryKind::Dir).count();
    let file_count = entries.iter().filter(|e| e.kind == EntryKind::File).count();
    let link_count = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Symlink)
        .count();

    let summary = build_summary(dir_count, file_count, link_count);
    lines.push(Line::from(vec![Span::styled(
        summary,
        Style::default().fg(Color::DarkGray),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        format!(
            " {:<10}  {:>8}  {:<16}  {}",
            "Permissions", "Size", "Modified", "Name"
        ),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" {}", "─".repeat(60)),
        Style::default().fg(Color::DarkGray),
    )]));

    for entry in &entries {
        lines.push(render_entry(entry));
    }

    Text::from(lines)
}

fn build_summary(dirs: usize, files: usize, links: usize) -> String {
    let mut parts = Vec::new();
    if dirs > 0 {
        parts.push(format!(
            "{} {}",
            dirs,
            if dirs == 1 {
                "directory"
            } else {
                "directories"
            }
        ));
    }
    if files > 0 {
        parts.push(format!(
            "{} {}",
            files,
            if files == 1 { "file" } else { "files" }
        ));
    }
    if links > 0 {
        parts.push(format!(
            "{} {}",
            links,
            if links == 1 { "symlink" } else { "symlinks" }
        ));
    }
    if parts.is_empty() {
        "  empty".to_string()
    } else {
        format!("  {}", parts.join(", "))
    }
}

#[derive(PartialEq, Eq)]
enum EntryKind {
    Dir,
    File,
    Symlink,
}

struct DirEntry {
    name: String,
    kind: EntryKind,
    size: Option<u64>,
    mtime: Option<u64>,
    permissions: String,
    link_target: Option<String>,
}

fn collect_entries(path: &Path) -> std::io::Result<Vec<DirEntry>> {
    let mut entries: Vec<DirEntry> = Vec::new();

    for result in fs::read_dir(path)? {
        let dir_entry = result?;
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        let meta = dir_entry.path().symlink_metadata()?;
        let file_type = meta.file_type();

        let kind = if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };

        let size = if file_type.is_dir() {
            None
        } else {
            Some(meta.len())
        };

        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let permissions = format_permissions(&meta);

        let link_target = if file_type.is_symlink() {
            fs::read_link(dir_entry.path())
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };

        entries.push(DirEntry {
            name,
            kind,
            size,
            mtime,
            permissions,
            link_target,
        });
    }

    entries.sort_by(|a, b| {
        let order = |k: &EntryKind| match k {
            EntryKind::Dir => 0,
            EntryKind::Symlink => 1,
            EntryKind::File => 2,
        };
        order(&a.kind)
            .cmp(&order(&b.kind))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

fn render_entry(entry: &DirEntry) -> Line<'static> {
    let perm_str = format!(" {:<10}  ", entry.permissions);
    let size_str = match entry.size {
        Some(s) => format!("{:>8}  ", format_size_short(s)),
        None => format!("{:>8}  ", "-"),
    };
    let mtime_str = match entry.mtime {
        Some(t) => format!("{:<16}  ", format_unix_timestamp(t)),
        None => format!("{:<16}  ", "Unknown"),
    };
    let name_str = match (&entry.kind, &entry.link_target) {
        (EntryKind::Dir, _) => format!("{}/", entry.name),
        (EntryKind::Symlink, Some(target)) => format!("{} -> {}", entry.name, target),
        (EntryKind::Symlink, None) => format!("{}@", entry.name),
        (EntryKind::File, _) => entry.name.clone(),
    };

    let name_color = match entry.kind {
        EntryKind::Dir => Color::Cyan,
        EntryKind::Symlink => Color::Magenta,
        EntryKind::File => name_color_for_file(&entry.name, &entry.permissions),
    };

    Line::from(vec![
        Span::styled(perm_str, Style::default().fg(Color::DarkGray)),
        Span::styled(size_str, Style::default().fg(Color::Yellow)),
        Span::styled(mtime_str, Style::default().fg(Color::DarkGray)),
        Span::styled(name_str, Style::default().fg(name_color)),
    ])
}

fn format_size_short(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}", size)
    }
}

#[cfg(unix)]
fn format_permissions(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    let mode = meta.permissions().mode();
    let ft = meta.file_type();
    let type_char = if ft.is_symlink() {
        'l'
    } else if ft.is_dir() {
        'd'
    } else {
        '-'
    };
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let mut s = String::with_capacity(10);
    s.push(type_char);
    for &(bit, ch) in &bits {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

#[cfg(not(unix))]
fn format_permissions(meta: &fs::Metadata) -> String {
    let ft = meta.file_type();
    let type_char = if ft.is_symlink() {
        'l'
    } else if ft.is_dir() {
        'd'
    } else {
        '-'
    };
    // Windows: only read-only flag is meaningful.
    let rw = if meta.permissions().readonly() {
        "r--r--r--"
    } else {
        "rw-rw-rw-"
    };
    format!("{}{}", type_char, rw)
}

/// On Unix, color executable files green. Otherwise white.
fn name_color_for_file(name: &str, permissions: &str) -> Color {
    let _ = name;
    #[cfg(unix)]
    {
        // permissions[3] is the owner-execute bit position (index 3 in "drwxr-xr-x").
        if permissions.as_bytes().get(3).copied() == Some(b'x') {
            return Color::Green;
        }
    }
    let _ = permissions;
    Color::White
}
