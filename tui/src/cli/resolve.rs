use super::Cli;
use crate::runtime::config::ResolvedCli;

pub fn resolve_cli(cli: Cli, stdin_is_piped: bool) -> anyhow::Result<ResolvedCli> {
    ResolvedCli::from_cli(cli, stdin_is_piped)
}
