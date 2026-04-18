use std::process::Command;

use color_eyre::eyre::{Result, eyre};
use dialoguer::{Input, theme::ColorfulTheme};

use crate::{
    config::{Config, host::detect_hostname},
    linker,
    state::State,
};

#[derive(clap::Parser, Debug)]
pub struct SyncCommand {
    #[command(subcommand)]
    command: SyncCommands,
}

#[derive(clap::Subcommand, Debug)]
enum SyncCommands {
    /// Pull latest changes and re-link modules
    Pull,
    /// Commit and push local changes
    Push {
        #[arg(short, long)]
        message: Option<String>,
    },
}

impl SyncCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let dotfiles_dir = config.dotfiles_dir()?;

        match &self.command {
            SyncCommands::Pull => pull(&config, &dotfiles_dir),
            SyncCommands::Push { message } => push(
                &dotfiles_dir,
                message.as_deref(),
                config.commit_cmd().as_deref(),
            ),
        }
    }
}

fn pull(config: &Config, dotfiles_dir: &std::path::Path) -> Result<()> {
    println!("Pulling latest changes...");
    git(&["pull"], dotfiles_dir)?;

    let mut state = State::load()?;
    let hostname = detect_hostname();

    let profile_name = match state.active_profile.clone() {
        Some(p) => p,
        None => {
            println!("No active profile — skipping re-link.");
            return Ok(());
        }
    };

    let profile = config
        .profiles
        .get(&profile_name)
        .ok_or_else(|| eyre!("profile '{}' not found in config", profile_name))?;

    println!("Re-linking profile '{}'...", profile_name);
    for module_name in &profile.modules {
        println!("Module '{}':", module_name);
        linker::link_module(
            config,
            &mut state,
            module_name,
            Some(&profile_name),
            &hostname,
        )?;
    }

    state.last_sync = Some(chrono::Local::now().to_rfc3339());
    state.save()?;

    Ok(())
}

fn push(
    dotfiles_dir: &std::path::Path,
    message: Option<&str>,
    commit_cmd: Option<&str>,
) -> Result<()> {
    git(&["add", "-A"], dotfiles_dir)?;

    if let Some(cmd) = commit_cmd {
        let mut parts = cmd.split_whitespace();
        let bin = parts
            .next()
            .ok_or_else(|| eyre!("commit_cmd cannot be empty"))?;
        let args: Vec<&str> = parts.collect();

        let status = Command::new(bin)
            .args(args)
            .current_dir(dotfiles_dir)
            .status()
            .map_err(|e| eyre!("failed to run commit_cmd '{}': {}", cmd, e))?;

        if !status.success() {
            return Err(eyre!("commit_cmd '{}' failed", cmd));
        }
    } else {
        let msg = match message {
            Some(m) => m.to_string(),
            None => {
                let theme = ColorfulTheme::default();
                Input::with_theme(&theme)
                    .with_prompt("Commit message")
                    .interact_text()?
            }
        };
        git(&["commit", "-m", &msg], dotfiles_dir)?;
    }

    git(&["push"], dotfiles_dir)?;
    Ok(())
}

fn git(args: &[&str], dir: &std::path::Path) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| eyre!("failed to run git command: {}", e))?;

    if !status.success() {
        return Err(eyre!("git command failed with status: {}", status));
    }

    Ok(())
}
