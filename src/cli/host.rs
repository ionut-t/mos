use color_eyre::eyre::{self, Result, eyre};
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::{
    config::{Config, host},
    state::State,
};

#[derive(clap::Parser, Debug)]
pub struct HostCommand {
    #[command(subcommand)]
    command: HostCommands,
}

#[derive(clap::Subcommand, Debug)]
enum HostCommands {
    /// Register the current host with a default profile
    Register(RegisterCommand),
    /// List all registered hosts
    List(ListCommand),
    /// Show info about the current host
    Info(InfoCommand),
    /// Remove a registered host
    Remove(RemoveCommand),
}

impl HostCommand {
    pub fn run(&self, config_path: &std::path::Path) -> color_eyre::eyre::Result<()> {
        match &self.command {
            HostCommands::Register(cmd) => cmd.run(config_path),
            HostCommands::List(cmd) => cmd.run(config_path),
            HostCommands::Info(cmd) => cmd.run(config_path),
            HostCommands::Remove(cmd) => cmd.run(config_path),
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct RegisterCommand;

impl RegisterCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let theme = ColorfulTheme::default();

        let detected_hostname = host::detect_hostname();
        let hostname: String = dialoguer::Input::with_theme(&theme)
            .with_prompt("Hostname")
            .default(detected_hostname.clone())
            .interact_text()?;

        if config.hosts.contains_key(&hostname) {
            eyre::bail!("host '{}' is already registered", hostname);
        }

        let profile_names: Vec<&String> = config.profiles.keys().collect();

        if profile_names.is_empty() {
            eyre::bail!("no profiles found — create a profile first with `mos profile create`");
        }
        let profile_idx = Select::with_theme(&theme)
            .with_prompt("Select default profile for this host")
            .items(&profile_names)
            .default(0)
            .interact()?;

        let default_profile = profile_names[profile_idx].to_string();

        let os = host::detect_os();

        let pkg_manager_options = ["brew", "apt"];
        let default_pm = if os == "macos" { 0 } else { 1 };
        let pm_idx = Select::with_theme(&theme)
            .with_prompt("Package manager")
            .items(pkg_manager_options)
            .default(default_pm)
            .interact()?;
        let package_manager = pkg_manager_options[pm_idx];

        let raw = std::fs::read_to_string(config_path)?;
        let mut doc: DocumentMut = raw.parse()?;

        let mut host_table = Table::new();
        host_table["os"] = value(os);
        host_table["package_manager"] = value(package_manager);
        host_table["default_profile"] = value(default_profile.as_str());

        doc["hosts"][&hostname] = Item::Table(host_table);
        std::fs::write(config_path, doc.to_string())?;

        let mut state = State::load()?;
        state.hostname = hostname.clone();
        state.active_profile = Some(default_profile.clone());
        state.save()?;

        println!(
            "Registered host {} with profile {}",
            hostname, default_profile,
        );

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct ListCommand;

impl ListCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let state = State::load()?;

        if config.hosts.is_empty() {
            println!("No hosts registered - use 'dotm host register' to add this machine");
            return Ok(());
        }

        for (name, host) in &config.hosts {
            let current = state.hostname == *name;
            let marker = if current { "*" } else { " " };
            let profile = host.default_profile.as_deref().unwrap_or("none");
            println!("{} [{}]  ({})", marker, name, profile);
        }

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct InfoCommand;

impl InfoCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let hostname = host::detect_hostname();

        let host = config.hosts.get(&hostname).ok_or_else(|| {
            eyre::eyre!(
                "host '{}' is not registered. Use 'dotm host register' to add this machine",
                hostname
            )
        })?;

        println!("Hostname:        {}", hostname);
        println!("OS:              {}", host.os);
        println!(
            "Package manager: {}",
            host.package_manager
                .as_ref()
                .map(|pm| match pm {
                    host::PackageManager::Brew => "brew",
                    host::PackageManager::Apt => "apt",
                })
                .unwrap_or("auto-detect")
        );
        println!(
            "Profile:         {}",
            host.default_profile.as_deref().unwrap_or("none")
        );

        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
pub struct RemoveCommand;

impl RemoveCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let theme = ColorfulTheme::default();

        let hostnames: Vec<&String> = config.hosts.keys().collect();

        if hostnames.is_empty() {
            println!("No hosts registered");
            return Ok(());
        }

        let idx = Select::with_theme(&theme)
            .with_prompt("Select host to remove")
            .items(&hostnames)
            .default(0)
            .interact()?;

        let hostname = hostnames[idx].clone();

        let confirmed = Confirm::with_theme(&theme)
            .with_prompt(format!("Remove host '{}'?", hostname))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }

        let raw = std::fs::read_to_string(config_path)?;
        let mut doc: DocumentMut = raw.parse()?;

        doc["hosts"]
            .as_table_mut()
            .ok_or_else(|| eyre!("no [hosts] table found in mos.toml"))?
            .remove(&hostname);

        std::fs::write(config_path, doc.to_string())?;

        // Clear state if removing the current machine
        let mut state = State::load()?;
        if state.hostname == hostname {
            state.hostname = String::new();
            state.active_profile = None;
            state.save()?;
        }

        println!("Removed host '{}'.", hostname);

        Ok(())
    }
}
