use crate::app::AppEvent::{Input, LogAppend, Preview};
use crate::app::{App, AppEvent, Mode};
use crate::config::{LoadedAppConfig, PersistedLayout};
use crate::infra::channel::{self, MapSender, Sender};
use crate::infra::terminal::TerminalSessionGuard;
use crate::output::render_selection_outputs;
use crate::preview::{self, structured_log, PreviewRequest, PreviewSource};
use crate::runtime::config::RunConfig;
use crate::runtime::interactive::input::spawn_input_handler;
use crate::runtime::interactive::r#loop::run_event_loop;
use crate::runtime::startup::{self, StartupMode};
use crate::search::controller::SearchController;
use crate::search::types::SearchConfig;
use crossterm::tty::IsTty;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self};

pub fn run_interactive_with_configs(
    run_config: RunConfig,
    search_config: SearchConfig,
    app_config: LoadedAppConfig,
    persisted_layout: PersistedLayout,
    log_max_entries: usize,
) -> anyhow::Result<()> {
    let prepared_input = startup::prepare_interactive_input_with_run_config(&run_config)?;
    let mut terminal_session = TerminalSessionGuard::enter()?;
    let mut terminal = build_terminal()?;
    let picker = build_picker();

    let (tx_main, rx_main) = channel::unbounded_default::<AppEvent>();
    let (tx_preview_req, rx_preview_req) = channel::unbounded_default::<PreviewRequest>();
    let (tx_cmd_noop, _rx_cmd_noop) = channel::unbounded_default();

    let tx_input = MapSender::new(tx_main.clone(), Input);
    let tx_preview_resp = MapSender::new(tx_main.clone(), |(source, content)| {
        Preview(source, content)
    });
    let tx_log = MapSender::new(tx_main.clone(), |(path, entries)| LogAppend(path, entries));

    let startup_mode = startup::classify_input_mode_with_run_config(&run_config);
    let mut search_sessions = if matches!(startup_mode, StartupMode::InteractiveDirectDiff) {
        None
    } else {
        Some(SearchController::from_search_config(
            search_config.clone(),
            prepared_input.stdin_items,
            &tx_main,
            !run_config.log,
        ))
    };
    spawn_input_handler(tx_input);
    preview::spawn_previewer(
        rx_preview_req,
        tx_preview_resp,
        tx_log.clone(),
        picker,
        run_config.preview_command.clone(),
        run_config.preview_delimiter.clone(),
        log_max_entries,
    );

    if let Some(pipe) = prepared_input.log_pipe {
        let _ = startup::spawn_log_stdin_reader(pipe, tx_log.clone());
    }

    if !prepared_input.log_files.is_empty() {
        startup::spawn_log_file_watchers(&prepared_input.log_files, tx_log);
    }

    let mut app = App::from_configs(run_config, search_config, app_config);
    initialize_app(&mut app, &persisted_layout, terminal.size()?.into());
    prime_search_log_and_diff_state(&mut app, search_sessions.as_ref(), &tx_preview_req);

    run_event_loop(
        &mut app,
        &mut terminal,
        &mut terminal_session,
        &rx_main,
        &tx_preview_req,
        &tx_cmd_noop,
        &mut search_sessions,
        log_max_entries,
    )?;

    crate::config::save_layout(&PersistedLayout {
        panes_swapped: app.ui.layout.panes_swapped,
        preview_percent: app.ui.layout.preview_percent,
        search_bar_at_bottom: app.ui.layout.search_bar_at_bottom,
        preview_hidden: app.ui.layout.preview_hidden,
    });

    if let Some(search_sessions) = search_sessions.as_mut() {
        search_sessions.shutdown();
    }
    let selected_output = app.take_selected_output();
    let rendered_output = render_selection_outputs(&selected_output, app.runtime.run.output_format);
    drop(terminal);
    drop(terminal_session);

    if let Some(output) = rendered_output {
        println!("{}", output);
    }

    Ok(())
}

fn build_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stderr>>> {
    let backend = CrosstermBackend::new(io::stderr());
    Ok(Terminal::new(backend)?)
}

fn build_picker() -> ratatui_image::picker::Picker {
    let mut picker = if io::stdout().is_tty() {
        ratatui_image::picker::Picker::from_query_stdio()
            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks())
    } else {
        ratatui_image::picker::Picker::halfblocks()
    };

    if std::env::var("TERM_PROGRAM").unwrap_or_default() == "iTerm.app"
        || std::env::var("LC_TERMINAL").unwrap_or_default() == "iTerm2"
    {
        picker.set_protocol_type(ratatui_image::picker::ProtocolType::Iterm2);
    }

    picker
}

fn initialize_app(
    app: &mut App,
    persisted_layout: &PersistedLayout,
    terminal_area: ratatui::layout::Rect,
) {
    app.ui.layout.panes_swapped = persisted_layout.panes_swapped;
    app.ui.layout.preview_percent = persisted_layout.preview_percent;
    app.ui.layout.search_bar_at_bottom = persisted_layout.search_bar_at_bottom;
    app.ui.layout.preview_hidden = persisted_layout.preview_hidden;
    app.set_terminal_area(terminal_area);
    app.refresh_viewports();
}

fn prime_search_log_and_diff_state(
    app: &mut App,
    search_sessions: Option<&SearchController>,
    tx_preview_req: &channel::DefaultSender<PreviewRequest>,
) {
    if app.runtime.run.log {
        let path = if app.runtime.run.log_files.is_empty() {
            structured_log::STDIN_STREAM_PATH.to_string()
        } else {
            app.runtime.run.log_files[0].display().to_string()
        };
        structured_log::initialize_empty_stream(app, path, structured_log::LogFormat::Jsonl);
        return;
    }

    if let Some(diff_paths) = app.runtime.run.diff.as_ref() {
        let [left, right] = diff_paths;
        let left = left.display().to_string();
        let right = right.display().to_string();
        let source = PreviewSource::Diff {
            left: left.clone(),
            right: right.clone(),
        };
        app.preview_session.preview.source = Some(source.clone());
        app.ui.mode = Mode::Preview;
        app.ui.layout.preview_fullscreen = true;
        let _ = tx_preview_req.send(PreviewRequest::Diff {
            source,
            left,
            right,
        });
        return;
    }

    if !app.search_session.query.text.is_empty() {
        if let Some(search_sessions) = search_sessions {
            search_sessions.send_query(&app.search_session.query.text);
        }
    }
}
