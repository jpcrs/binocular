use binocular::cli;
use binocular::runtime::interactive;
use binocular::runtime::{headless, startup};

use clap::Parser;
use std::io::{self, IsTerminal};

fn main() -> anyhow::Result<()> {
    let resolved = cli::resolve_cli(cli::Cli::parse(), !io::stdin().is_terminal())?;
    let run_config = resolved.run;
    let search_config = resolved.search;
    let app_config = binocular::config::load_app_config();
    let persisted_layout = binocular::config::load_layout();
    let log_max_entries = app_config.log.max_entries;

    if run_config.headless {
        let stdin_items = startup::prepare_headless_input_with_run_config(&run_config)?;
        return headless::run_with_configs(run_config, search_config, stdin_items);
    }

    interactive::run_interactive_with_configs(
        run_config,
        search_config,
        app_config,
        persisted_layout,
        log_max_entries,
    )
}
