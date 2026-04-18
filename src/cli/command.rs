use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};
use std::path::{Path, PathBuf};

use crate::cli::host::HostCommand;
use crate::cli::sync::SyncCommand;
use crate::cli::{
    deps::DepsCommand, init::InitCommand, link::LinkCommand, profile::ProfileCommand,
    status::StatusCommand, unlink::UnlinkCommand,
};
use crate::global_config::GlobalConfig;

#[derive(Parser)]
#[command(name = "mos", about = "Manage dotfiles across machines and profiles")]
pub struct Cli {
    /// Path to mos.toml (overrides global config)
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new dotfiles repository
    Init(InitCommand),
    /// Link a module (or all modules in the active profile)
    Link(LinkCommand),
    /// Unlink a module
    Unlink(UnlinkCommand),
    /// Show current status
    Status(StatusCommand),
    /// Profile management
    Profile(ProfileCommand),
    /// Host management
    Host(HostCommand),
    /// Dependency management
    Deps(DepsCommand),
    /// Sync with remote repository
    Sync(SyncCommand),
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        // Init doesn't need a config path
        if let Commands::Init(cmd) = &self.command {
            return cmd.run();
        }

        let config_path = resolve_config_path(self.config.as_deref())?;

        match &self.command {
            Commands::Init(_) => unreachable!(),
            Commands::Link(cmd) => cmd.run(&config_path),
            Commands::Unlink(cmd) => cmd.run(),
            Commands::Status(cmd) => cmd.run(&config_path),
            Commands::Profile(cmd) => cmd.run(&config_path),
            Commands::Host(cmd) => cmd.run(&config_path),
            Commands::Deps(cmd) => cmd.run(&config_path),
            Commands::Sync(cmd) => cmd.run(&config_path),
        }
    }
}

fn resolve_config_path(flag: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = flag {
        return Ok(p.to_path_buf());
    }

    if let Some(gc) = GlobalConfig::load() {
        let dotfiles_dir = gc.dotfiles_dir()?;
        return Ok(dotfiles_dir.join("mos.toml"));
    }

    // Fallback: mos.toml in current directory
    let cwd = std::env::current_dir()?.join("mos.toml");
    if cwd.exists() {
        return Ok(cwd);
    }

    Err(eyre!(
        "no dotfiles directory configured — run `mos init` first"
    ))
}
