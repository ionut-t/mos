mod cli;
mod config;
mod deps;
mod global_config;
mod linker;
mod path;
mod state;

use clap::Parser;
use color_eyre::eyre::Result;

use crate::cli::command::Cli;

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_location_section(false)
        .display_env_section(false)
        .install()?;

    let cli = Cli::parse();

    cli.run()
}
