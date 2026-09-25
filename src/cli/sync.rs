use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{Result, eyre};
use dialoguer::{Input, theme::ColorfulTheme};

use crate::{
    config::{Config, host::detect_hostname},
    linker,
    state::State,
    ui,
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
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let dotfiles_dir = config.dotfiles_dir()?;
        let state = State::load()?;

        match &self.command {
            SyncCommands::Pull => pull(&config, &dotfiles_dir, state),
            SyncCommands::Push { message } => push(
                &dotfiles_dir,
                message.as_deref(),
                config.commit_cmd().as_deref(),
                &config,
            ),
        }
    }
}

fn pull(config: &Config, dotfiles_dir: &Path, mut state: State) -> Result<()> {
    ui::step("Pulling latest changes...");
    git(&["pull"], dotfiles_dir)?;

    let hostname = detect_hostname();

    let profile_name = match state.active_profile.clone() {
        Some(p) => p,
        None => {
            ui::warn("No active profile — skipping re-link.");
            return Ok(());
        }
    };

    let profile = config
        .profiles
        .get(&profile_name)
        .ok_or_else(|| eyre!("profile '{}' not found in config", profile_name))?;

    ui::step(format!("Re-linking profile '{}'...", profile_name));
    for module_name in &profile.modules {
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
    dotfiles_dir: &Path,
    message: Option<&str>,
    commit_cmd: Option<&str>,
    config: &Config,
) -> Result<()> {
    reconcile_gitignore(dotfiles_dir, config)?;

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
    ui::success("Pushed.");
    Ok(())
}

/// Sync .gitignore and git index with tracked settings.
/// Iterates all profiles and all modules so the result is correct regardless
/// of which profile is currently active on this machine.
fn reconcile_gitignore(dotfiles_dir: &Path, config: &Config) -> Result<()> {
    let gitignore_path = dotfiles_dir.join(".gitignore");

    let existing = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

    for (profile_name, profile) in &config.profiles {
        let profile_pattern = format!("profiles/{}/", profile_name);

        if !profile.tracked {
            if !lines.contains(&profile_pattern) {
                lines.push(profile_pattern);
            }
            let profile_dir = dotfiles_dir.join("profiles").join(profile_name);
            if profile_dir.exists() {
                evict_paths(dotfiles_dir, vec![profile_dir])?;
            }
        } else {
            lines.retain(|l| l != &profile_pattern);

            for module_name in &profile.modules {
                let Some(module) = config.modules.get(module_name) else {
                    continue;
                };
                let Some(ref source) = module.source else {
                    continue;
                };
                let seg = source.split('/').next().unwrap_or(source.as_str());
                let pattern = format!("profiles/{}/{}/", profile_name, seg);

                if profile.untracked.contains(module_name) {
                    if !lines.contains(&pattern) {
                        lines.push(pattern.clone());
                    }
                    let profile_path = dotfiles_dir.join("profiles").join(profile_name).join(seg);
                    if profile_path.exists() {
                        evict_paths(dotfiles_dir, vec![profile_path])?;
                    }
                } else {
                    lines.retain(|l| l != &pattern);
                }
            }
        }
    }

    for module in config.modules.values() {
        let Some(ref source) = module.source else {
            continue;
        };
        let seg = source.split('/').next().unwrap_or(source.as_str());

        if !module.tracked {
            for pattern in &[
                format!("base/{}/", seg),
                format!("profiles/*/{}/", seg),
                format!("hosts/*/{}/", seg),
            ] {
                if !lines.contains(pattern) {
                    lines.push(pattern.clone());
                }
            }
            evict_paths(dotfiles_dir, module_layer_paths(dotfiles_dir, seg))?;
        } else {
            for pattern in &[
                format!("base/{}/", seg),
                format!("profiles/*/{}/", seg),
                format!("hosts/*/{}/", seg),
            ] {
                lines.retain(|l| l != pattern);
            }
        }
    }

    std::fs::write(&gitignore_path, lines.join("\n") + "\n")?;
    Ok(())
}

fn evict_paths(dotfiles_dir: &Path, paths: Vec<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let rel: Vec<String> = paths
        .iter()
        .filter_map(|p| p.strip_prefix(dotfiles_dir).ok())
        .map(|p| p.display().to_string())
        .collect();
    let mut args = vec!["rm", "--cached", "-r", "--ignore-unmatch", "--"];
    args.extend(rel.iter().map(|s| s.as_str()));
    git(&args, dotfiles_dir)
}

/// Collect existing paths for a module segment across base/, profiles/*, and hosts/*.
fn module_layer_paths(dotfiles_dir: &Path, seg: &str) -> Vec<PathBuf> {
    let mut paths = vec![];

    let base = dotfiles_dir.join("base").join(seg);
    if base.exists() {
        paths.push(base);
    }

    for layer_dir in [dotfiles_dir.join("profiles"), dotfiles_dir.join("hosts")] {
        if let Ok(entries) = std::fs::read_dir(&layer_dir) {
            for entry in entries.flatten() {
                let p = entry.path().join(seg);
                if p.exists() {
                    paths.push(p);
                }
            }
        }
    }

    paths
}

fn git(args: &[&str], dir: &Path) -> Result<()> {
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
