use crate::app::{App, HelpTab};
use crate::config::{format_keybindings, KeyBinding};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

struct HelpSection<'a> {
    title: &'a str,
    rows: Vec<HelpRow>,
}

enum HelpRow {
    Shortcut { keys: String, description: String },
    Text(String),
}

pub fn render_help_modal(f: &mut Frame, app: &App) {
    if !app.ui.help.visible {
        return;
    }

    let area = centered_rect(88, 86, f.area());
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Help", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
            ])
            .centered(),
        )
        .border_style(Style::default().fg(Color::LightCyan));

    f.render_widget(Clear, area);
    f.render_widget(outer.clone(), area);

    let inner = outer.inner(area);
    let [header_area, body_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .areas(inner);
    let [tabs_area, content_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(20)])
        .areas(body_area);

    render_header(f, app, header_area);
    render_tabs(f, app, tabs_area);
    render_content(f, app, content_area);
    render_footer(f, app, footer_area);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let [title_area, subtitle_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .areas(area);

    let title = app.ui.help.tab.title().to_string();
    let subtitle = match app.ui.help.tab {
        HelpTab::Overview => "Configured app shortcuts and how the help modal works",
        HelpTab::Search => "Search results, search bar editing, and result actions",
        HelpTab::Preview => "Preview focus, text editing, and read-only behavior",
        HelpTab::Logs => "Structured-log filtering, columns, and live navigation",
        HelpTab::Layout => "Preview visibility, pane arrangement, and window controls",
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )])),
        title_area,
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            subtitle,
            Style::default().fg(Color::Gray),
        )]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        subtitle_area,
    );
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tabs = [
        HelpTab::Overview,
        HelpTab::Search,
        HelpTab::Preview,
        HelpTab::Logs,
        HelpTab::Layout,
    ];

    let lines = tabs
        .iter()
        .enumerate()
        .map(|(idx, tab)| {
            let active = *tab == app.ui.help.tab;
            let style = if active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(vec![Span::styled(
                format!(" {}. {} ", idx + 1, tab.title()),
                style,
            )])
        })
        .collect::<Vec<_>>();

    let block = Block::default()
        .title(" Sections ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let sections = match app.ui.help.tab {
        HelpTab::Overview => overview_sections(app),
        HelpTab::Search => search_sections(app),
        HelpTab::Preview => preview_sections(app),
        HelpTab::Logs => logs_sections(app),
        HelpTab::Layout => layout_sections(app),
    };
    let lines = render_sections(&sections);
    let block = Block::default()
        .title(format!(" {} ", app.ui.help.tab.title()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let close = format_keybindings(&app.keybindings().toggle_help);
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(close, Style::default().fg(Color::LightCyan)),
        Span::styled(" or Esc close", Style::default().fg(Color::Gray)),
        Span::styled("  •  ", Style::default().fg(Color::DarkGray)),
        Span::styled("1-5", Style::default().fg(Color::LightCyan)),
        Span::styled(" jump tabs", Style::default().fg(Color::Gray)),
        Span::styled("  •  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab / Shift+Tab", Style::default().fg(Color::LightCyan)),
        Span::styled(" cycle", Style::default().fg(Color::Gray)),
    ]));
    f.render_widget(footer, area);
}

fn overview_sections(app: &App) -> Vec<HelpSection<'static>> {
    vec![HelpSection {
        title: "Configured App Shortcuts",
        rows: vec![
            shortcut(&app.keybindings().toggle_help, "toggle help"),
            shortcut(&app.keybindings().quit, "quit binocular"),
            shortcut(
                &app.keybindings().toggle_exact,
                "toggle fuzzy/exact matcher",
            ),
            shortcut(&app.keybindings().mode_path, "switch to path mode"),
            shortcut(&app.keybindings().mode_files, "switch to file-name mode"),
            shortcut(&app.keybindings().mode_grep, "switch to content mode"),
            shortcut(&app.keybindings().mode_dirs, "switch to directory mode"),
        ],
    }]
}

fn search_sections(app: &App) -> Vec<HelpSection<'static>> {
    vec![
        HelpSection {
            title: "Search Results",
            rows: vec![
                shortcut(
                    &app.keybindings().mark_result,
                    "mark or unmark selected result",
                ),
                shortcut(
                    &app.keybindings().mark_diff_result,
                    "mark or unmark result for diff",
                ),
                HelpRow::Shortcut {
                    keys: "Enter".to_string(),
                    description: "select current result and quit".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "j / k".to_string(),
                    description: "move selection in normal mode".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "Up / Down".to_string(),
                    description: "move selection in insert mode".to_string(),
                },
            ],
        },
        HelpSection {
            title: "Search Bar",
            rows: vec![
                HelpRow::Shortcut {
                    keys: "Type".to_string(),
                    description: "edit the query in insert mode".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "Esc".to_string(),
                    description: "leave insert mode".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "h / l / w / e / b / 0 / ^ / $".to_string(),
                    description: "vim cursor and word motions in normal mode".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "i / a / I / A".to_string(),
                    description: "enter insert mode variants".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "d/c + motion, diw, ciw".to_string(),
                    description: "use vim-style edit operators".to_string(),
                },
            ],
        },
    ]
}

fn preview_sections(app: &App) -> Vec<HelpSection<'static>> {
    vec![
        HelpSection {
            title: "Preview Actions",
            rows: vec![
                shortcut(&app.keybindings().toggle_preview_focus, "switch between search and preview"),
                shortcut(&app.keybindings().scroll_preview_up, "page preview upward"),
                shortcut(&app.keybindings().scroll_preview_down, "page preview downward"),
                shortcut(&app.keybindings().select_from_preview, "select highlighted location from preview"),
            ],
        },
        HelpSection {
            title: "Preview Vim Controls",
            rows: vec![
                HelpRow::Shortcut {
                    keys: "h / j / k / l".to_string(),
                    description: "move cursor".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "w / e / b / gg / G / % / f / F / ;".to_string(),
                    description: "navigate text quickly".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "v / V / y / d / c / u / Ctrl+R".to_string(),
                    description: "visual mode, yank, edit, undo, redo".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "/ / n / N / :w / :q / :wq".to_string(),
                    description: "search and command-line actions".to_string(),
                },
                HelpRow::Text(
                    "Read-only previews keep navigation but block focus/editing; the status line will say so when needed.".to_string(),
                ),
            ],
        },
    ]
}

fn logs_sections(_app: &App) -> Vec<HelpSection<'static>> {
    vec![
        HelpSection {
            title: "Structured Log Navigation",
            rows: vec![
                HelpRow::Shortcut {
                    keys: "j / k / Up / Down".to_string(),
                    description: "move between visible log rows".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "g / G / u / d".to_string(),
                    description: "jump newest, oldest, page up, page down".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "Tab".to_string(),
                    description: "mark current row".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "y / Y".to_string(),
                    description: "copy visible row or raw row".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "p".to_string(),
                    description: "pause or resume live updates".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "Esc / q".to_string(),
                    description: "leave the log viewer".to_string(),
                },
            ],
        },
        HelpSection {
            title: "Filtering and Columns",
            rows: vec![
                HelpRow::Shortcut {
                    keys: "/".to_string(),
                    description: "start editing the filter query".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "Esc / Enter".to_string(),
                    description: "leave filter editing".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "h / l / Left / Right".to_string(),
                    description: "move between visible columns".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "a".to_string(),
                    description: "open the column picker modal".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "H / o / < / >".to_string(),
                    description: "hide, isolate, or resize the selected column".to_string(),
                },
                HelpRow::Shortcut {
                    keys: "r".to_string(),
                    description: "reset filters, marks, and column layout".to_string(),
                },
                HelpRow::Text(
                    "Inside the column picker: j/k moves, Space toggles, Tab toggles and advances, Enter applies, Esc cancels.".to_string(),
                ),
            ],
        },
    ]
}

fn layout_sections(app: &App) -> Vec<HelpSection<'static>> {
    vec![
        HelpSection {
            title: "Window Layout",
            rows: vec![
                shortcut(&app.keybindings().toggle_preview_visibility, "show or hide the preview pane"),
                shortcut(&app.keybindings().toggle_preview_fullscreen, "toggle fullscreen preview"),
                shortcut(&app.keybindings().swap_panes, "swap results and preview columns"),
                shortcut(&app.keybindings().preview_wider, "make preview wider"),
                shortcut(&app.keybindings().preview_narrower, "make preview narrower"),
                shortcut(&app.keybindings().toggle_search_bar_position, "move search bar top or bottom"),
            ],
        },
        HelpSection {
            title: "Mode Notes",
            rows: vec![
                HelpRow::Text(
                    "Preview focus only works for editable text and structured-log previews.".to_string(),
                ),
                HelpRow::Text(
                    "Direct diff, git-backed previews, and many binary/plain previews stay read-only.".to_string(),
                ),
            ],
        },
    ]
}

fn render_sections(sections: &[HelpSection<'_>]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (idx, section) in sections.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(vec![Span::styled(
            section.title.to_string(),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "─".repeat(section.title.len().min(32)),
            Style::default().fg(Color::DarkGray),
        )]));
        for row in &section.rows {
            match row {
                HelpRow::Shortcut { keys, description } => {
                    let key_width = 26;
                    let key_len = keys.chars().count();
                    if key_len <= key_width {
                        let mut spans = Vec::new();
                        spans.push(Span::raw("  "));
                        spans.extend(render_key_spans(keys));
                        spans.push(Span::raw(" ".repeat(key_width.saturating_sub(key_len) + 1)));
                        spans.push(Span::styled(
                            description.clone(),
                            Style::default().fg(Color::Gray),
                        ));
                        lines.push(Line::from(spans));
                    } else {
                        let mut key_line = vec![Span::raw("  ")];
                        key_line.extend(render_key_spans(keys));
                        lines.push(Line::from(key_line));
                        lines.push(Line::from(vec![
                            Span::raw(" ".repeat(key_width + 3)),
                            Span::styled(description.clone(), Style::default().fg(Color::Gray)),
                        ]));
                    }
                }
                HelpRow::Text(text) => lines.push(Line::from(vec![Span::styled(
                    format!("  {text}"),
                    Style::default().fg(Color::DarkGray),
                )])),
            }
        }
    }
    lines
}

fn render_key_spans(keys: &str) -> Vec<Span<'static>> {
    let parts = keys.split(" / ").collect::<Vec<_>>();
    let mut spans = Vec::new();
    for (idx, part) in parts.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" / ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            (*part).to_string(),
            Style::default().fg(Color::LightCyan),
        ));
    }
    spans
}

fn shortcut(bindings: &[KeyBinding], description: &str) -> HelpRow {
    HelpRow::Shortcut {
        keys: format_keybindings(bindings),
        description: description.to_string(),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeyBinding, Keybindings};
    use crate::search::types::{MatcherMode, SearchConfig, SearchMode, SearchSettings};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn shortcut_rows_use_formatted_custom_bindings() {
        let row = shortcut(
            &[
                KeyBinding {
                    code: KeyCode::F(9),
                    modifiers: KeyModifiers::NONE,
                },
                KeyBinding {
                    code: KeyCode::Char('g'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ],
            "toggle",
        );
        let HelpRow::Shortcut { keys, description } = row else {
            panic!("expected shortcut row");
        };
        assert_eq!(keys, "F9 / Ctrl+G");
        assert_eq!(description, "toggle");
    }

    #[test]
    fn overview_section_uses_runtime_help_binding() {
        let mut keybindings = Keybindings::default();
        keybindings.toggle_help = vec![KeyBinding {
            code: KeyCode::F(12),
            modifiers: KeyModifiers::NONE,
        }];
        let app = App::from_configs(
            crate::runtime::config::RunConfig {
                headless: false,
                output_format: crate::cli::args::OutputFormat::Plain,
                output_file: None,
                stdin: false,
                log: false,
                diff: None,
                preview_command: None,
                preview_delimiter: ":".to_string(),
                split: None,
                log_files: Vec::new(),
            },
            SearchConfig {
                query: None,
                locations: vec![],
                search_pdf: false,
                no_hidden: false,
                no_git_ignore: false,
                no_ignore: false,
                no_default_ignore_dirs: false,
                git_search_scope: None,
                settings: SearchSettings {
                    mode: SearchMode::Path,
                    matcher: MatcherMode::Fuzzy,
                },
            },
            crate::config::LoadedAppConfig {
                keybindings,
                ..Default::default()
            },
        );
        let sections = overview_sections(&app);
        let rendered = render_sections(&sections)
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.into_owned()))
            .collect::<Vec<_>>()
            .join("");
        assert!(rendered.contains("F12"));
    }

    #[test]
    fn logs_section_documents_log_viewer_controls() {
        let rendered = render_sections(&logs_sections(&App::from_configs(
            crate::runtime::config::RunConfig {
                headless: false,
                output_format: crate::cli::args::OutputFormat::Plain,
                output_file: None,
                stdin: false,
                log: false,
                diff: None,
                preview_command: None,
                preview_delimiter: ":".to_string(),
                split: None,
                log_files: Vec::new(),
            },
            SearchConfig {
                query: None,
                locations: vec![],
                search_pdf: false,
                no_hidden: false,
                no_git_ignore: false,
                no_ignore: false,
                no_default_ignore_dirs: false,
                git_search_scope: None,
                settings: SearchSettings {
                    mode: SearchMode::Path,
                    matcher: MatcherMode::Fuzzy,
                },
            },
            crate::config::LoadedAppConfig::default(),
        )))
        .into_iter()
        .flat_map(|line| line.spans.into_iter().map(|span| span.content.into_owned()))
        .collect::<Vec<_>>()
        .join("");
        assert!(rendered.contains("column picker"));
        assert!(rendered.contains("pause or resume live updates"));
    }
}
