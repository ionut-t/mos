use std::path::PathBuf;

use color_eyre::eyre::{Result, eyre};
use dialoguer::{Confirm, Input, theme::ColorfulTheme};

use crate::config::host::{detect_hostname, detect_os};
use crate::global_config::GlobalConfig;
use crate::ui;

#[derive(clap::Parser, Debug)]
pub struct InitCommand {}

impl InitCommand {
    pub fn run(&self) -> Result<()> {
        let theme = ColorfulTheme::default();

        let default_path = dirs::home_dir()
            .map(|h| h.join(".dotfiles").display().to_string())
            .unwrap_or_else(|| "~/.dotfiles".to_string());

        let raw: String = Input::with_theme(&theme)
            .with_prompt("Dotfiles directory")
            .default(default_path)
            .interact_text()?;

        let base = expand_input_path(&raw)?;

        println!();
        ui::detail(format!("Location: {}", base.display()));
        println!();

        let confirmed = Confirm::with_theme(&theme)
            .with_prompt("Initialize here?")
            .default(true)
            .interact()?;

        if !confirmed {
            ui::warn("Cancelled.");
            return Ok(());
        }

        println!();

        let dirs = ["base", "profiles", "hosts", ".dotm"];
        for dir in &dirs {
            let dir_path = base.join(dir);
            std::fs::create_dir_all(&dir_path)
                .map_err(|e| eyre!("failed to create {}: {}", dir_path.display(), e))?;
            ui::item(format!("Created {}/", dir));
        }

        let config_path = base.join("mos.toml");
        if !config_path.exists() {
            let starter = generate_starter_config(&base);
            std::fs::write(&config_path, starter)
                .map_err(|e| eyre!("failed to write {}: {}", config_path.display(), e))?;
            ui::item("Created mos.toml");
        } else {
            ui::item("mos.toml already exists, skipping");
        }

        GlobalConfig::save(&base)?;
        ui::item("Saved global config (~/.config/mos/config.toml)");

        println!();
        ui::success(format!(
            "Dotfiles repository initialized at {}",
            base.display()
        ));
        println!();
        ui::step("Next steps:");
        ui::item("1. Edit mos.toml to define your modules and profiles");
        ui::item("2. Add config files under base/<module>/");
        ui::item("3. Run `mos link <module>` to create symlinks");

        Ok(())
    }
}

fn expand_input_path(raw: &str) -> Result<PathBuf> {
    let path = if let Some(rest) = raw.strip_prefix("~/") {
        dirs::home_dir()
            .ok_or_else(|| eyre!("could not determine home directory"))?
            .join(rest)
    } else if raw == "~" {
        dirs::home_dir().ok_or_else(|| eyre!("could not determine home directory"))?
    } else {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir()?.join(p)
        }
    };

    Ok(path)
}

fn generate_starter_config(base: &std::path::Path) -> String {
    let hostname = detect_hostname();
    let os = detect_os();

    format!(
        r#"[settings]
dotfiles_dir = "{dotfiles_dir}"
backup_dir = "~/.mos-backups"
backup_format = "{{name}}.{{datetime}}.bak"

# Define hosts
# [hosts.{hostname}]
# os = "{os}"
# default_profile = "default"

# Define profiles
# [profiles.default]
# description = "Default profile"
# modules = ["example"]

# Define modules
# [modules.example]
# source = "example"
# target = "~/.config/example"
"#,
        dotfiles_dir = base.display(),
        hostname = hostname,
        os = os,
    )
}
