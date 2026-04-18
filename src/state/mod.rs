pub mod diff;

use std::collections::HashMap;
use std::path::PathBuf;

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub hostname: String,
    pub os: String,
    pub active_profile: Option<String>,
    pub last_sync: Option<String>,
    #[serde(default)]
    pub links: HashMap<String, LinkState>,
    #[serde(default)]
    pub backups: HashMap<String, String>,
    #[serde(default)]
    pub deps_installed: InstalledDeps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkState {
    pub target: String,
    pub source_layer: String,
    pub files: Vec<LinkedFile>,
    pub linked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedFile {
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstalledDeps {
    #[serde(default)]
    pub brew: Vec<String>,
    #[serde(default)]
    pub apt: Vec<String>,
    #[serde(default)]
    pub go: Vec<String>,
    #[serde(default)]
    pub cargo: Vec<String>,
}

impl State {
    pub fn state_path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .ok_or_else(|| eyre!("could not determine XDG_DATA_HOME"))?;
        Ok(base.join("mos").join("state.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::state_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| eyre!("failed to read state at {}: {}", path.display(), e))?;
        let state: State = toml::from_str(&content)
            .map_err(|e| eyre!("failed to parse state: {}", e))?;
        Ok(state)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| eyre!("failed to create state directory: {}", e))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| eyre!("failed to serialize state: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| eyre!("failed to write state to {}: {}", path.display(), e))?;
        Ok(())
    }
}
