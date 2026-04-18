use color_eyre::eyre::{self, Result, eyre};
use dialoguer::{Input, MultiSelect, theme::ColorfulTheme};
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item, Table, value};

use crate::{
    config::{Config, host},
    linker,
    state::State,
};

#[derive(clap::Parser, Debug)]
pub struct ProfileCommand {
    #[command(subcommand)]
    command: ProfileCommands,
}

#[derive(clap::Subcommand, Debug)]
enum ProfileCommands {
    /// List all profiles
    List(ListCommand),
    /// Switch to a different profile
    Switch(SwitchCommand),
    /// Create a new profile
    Create(CreateCommand),
    /// Override a module for the active profile
    Override(OverrideCommand),
}

impl ProfileCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        match &self.command {
            ProfileCommands::List(cmd) => cmd.run(config_path),
            ProfileCommands::Switch(cmd) => cmd.run(config_path),
            ProfileCommands::Create(cmd) => cmd.run(config_path),
            ProfileCommands::Override(cmd) => cmd.run(config_path),
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct ListCommand;

impl ListCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let state = State::load()?;

        if config.profiles.is_empty() {
            println!("No profiles configured.");
            return Ok(());
        }

        for (name, profile) in &config.profiles {
            let active = state.active_profile.as_deref() == Some(name);
            let active_marker = if active { "*" } else { " " };

            println!(
                "{} {} ({} modules)",
                active_marker,
                name,
                profile.modules.len()
            );
        }

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct SwitchCommand {
    /// Name of the profile to switch to
    name: String,
}

impl SwitchCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let mut state = State::load()?;
        let hostname = host::detect_hostname();

        if !config.profiles.contains_key(&self.name) {
            eyre::bail!("profile '{}' does not exist", self.name);
        }

        if state.active_profile.as_deref() == Some(&self.name) {
            println!("Already on profile '{}'", self.name);
            return Ok(());
        }

        // Unlink current profile
        if let Some(ref current_profile_name) = state.active_profile
            && let Some(profile) = config.profiles.get(current_profile_name)
        {
            println!("Unlinking current profile '{}'...", current_profile_name);
            for module_name in &profile.modules {
                if state.links.contains_key(module_name) {
                    linker::unlink_module(&mut state, module_name)?;
                }
            }
        }

        // Link new profile
        let profile = &config.profiles[&self.name];
        println!("Linking new profile '{}'...", self.name);
        for module_name in &profile.modules {
            if !state.links.contains_key(module_name) {
                linker::link_module(
                    &config,
                    &mut state,
                    module_name,
                    Some(&self.name),
                    &hostname,
                )?;
            }
        }

        state.active_profile = Some(self.name.clone());
        state.save()?;
        println!("Switched to profile '{}'", self.name);

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct CreateCommand {
    /// Profile name
    name: String,
    /// Clone modules from an existing profile
    #[arg(long)]
    from: Option<String>,
}

impl CreateCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let theme = ColorfulTheme::default();

        if config.profiles.contains_key(&self.name) {
            eyre::bail!("profile '{}' already exists", self.name);
        }

        let description: String = Input::with_theme(&theme)
            .with_prompt("Description")
            .allow_empty(true)
            .interact_text()?;

        // Collect available modules, pre-selecting from --from profile if given
        let mut module_names: Vec<&String> = config.modules.keys().collect();
        module_names.sort();

        let preselected: Vec<bool> = if let Some(ref from) = self.from {
            let from_profile = config
                .profiles
                .get(from)
                .ok_or_else(|| eyre!("profile '{}' not found", from))?;
            module_names
                .iter()
                .map(|m| from_profile.modules.contains(m))
                .collect()
        } else {
            vec![false; module_names.len()]
        };

        let items: Vec<&str> = module_names.iter().map(|m| m.as_str()).collect();
        let selections = MultiSelect::with_theme(&theme)
            .with_prompt("Select modules")
            .items(&items)
            .defaults(&preselected)
            .interact()?;

        let selected_modules: Vec<&str> = selections.iter().map(|&i| items[i]).collect();

        // Write to mos.toml using toml_edit
        let raw = std::fs::read_to_string(config_path)
            .map_err(|e| eyre!("failed to read {}: {}", config_path.display(), e))?;
        let mut doc: DocumentMut = raw
            .parse()
            .map_err(|e| eyre!("failed to parse mos.toml: {}", e))?;

        let mut profile_table = Table::new();
        if !description.is_empty() {
            profile_table["description"] = value(description);
        }
        profile_table["tracked"] = value(true);

        let mut modules_array = Array::new();
        for m in &selected_modules {
            modules_array.push(*m);
        }
        profile_table["modules"] = Item::Value(modules_array.into());

        doc["profiles"][&self.name] = Item::Table(profile_table);

        std::fs::write(config_path, doc.to_string())
            .map_err(|e| eyre!("failed to write mos.toml: {}", e))?;

        // Create the profile directory in the dotfiles repo
        let config = Config::load(config_path)?;
        let profile_dir = config.dotfiles_dir()?.join("profiles").join(&self.name);
        std::fs::create_dir_all(&profile_dir)
            .map_err(|e| eyre!("failed to create profile directory: {}", e))?;

        println!("Created profile '{}'.", self.name);

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct OverrideCommand {
    /// Module to override
    module: String,
}

impl OverrideCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let state = State::load()?;

        let profile_name = state
            .active_profile
            .as_deref()
            .ok_or_else(|| eyre!("no active profile — run `mos profile switch <name>` first"))?;

        if !config.modules.contains_key(&self.module) {
            eyre::bail!("module '{}' does not exist", self.module);
        }

        let module = &config.modules[&self.module];
        let source = module
            .source
            .as_deref()
            .ok_or_else(|| eyre!("module '{}' has no source — it is deps-only", self.module))?;
        let dotfiles_dir = config.dotfiles_dir()?;

        let base_source = dotfiles_dir.join("base").join(source);
        let dest = dotfiles_dir
            .join("profiles")
            .join(profile_name)
            .join(source);

        if dest.exists() {
            eyre::bail!("override already exists at {}", dest.display());
        }

        if base_source.is_file() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&base_source, &dest)?;
        } else if base_source.is_dir() {
            copy_dir(&base_source, &dest)?;
        } else {
            eyre::bail!("no base source found for module '{}'", self.module);
        }

        println!("Created override at {}", dest.display());
        println!(
            "Edit the files there, then run `mos link {}` to apply.",
            self.module
        );

        Ok(())
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
