pub mod host;
pub mod module;
pub mod profile;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

pub use host::Host;
pub use module::Module;
pub use profile::Profile;

use crate::path::expand_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(default)]
    pub hosts: HashMap<String, Host>,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default)]
    pub modules: HashMap<String, Module>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub dotfiles_dir: String,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_backup_format")]
    pub backup_format: String,
}

fn default_backup_dir() -> String {
    "~/.mos-backups".to_string()
}

fn default_backup_format() -> String {
    "{name}.{datetime}.bak".to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| eyre!("failed to read config at {}: {}", path.display(), e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| eyre!("failed to parse config: {}", e))?;
        config.validate()?;
        Ok(config)
    }

    pub fn dotfiles_dir(&self) -> Result<PathBuf> {
        expand_path(&self.settings.dotfiles_dir)
    }

    pub fn backup_dir(&self) -> Result<PathBuf> {
        expand_path(&self.settings.backup_dir)
    }

    fn validate(&self) -> Result<()> {
        // All modules referenced by profiles must exist
        for (profile_name, profile) in &self.profiles {
            for module_name in &profile.modules {
                if !self.modules.contains_key(module_name) {
                    return Err(eyre!(
                        "profile '{}' references non-existent module '{}'",
                        profile_name,
                        module_name
                    ));
                }
            }
        }

        // All default_profile values must reference existing profiles
        for (host_name, host) in &self.hosts {
            if let Some(ref dp) = host.default_profile
                && !self.profiles.contains_key(dp)
            {
                return Err(eyre!(
                    "host '{}' references non-existent default_profile '{}'",
                    host_name,
                    dp
                ));
            }
        }

        Ok(())
    }
}
