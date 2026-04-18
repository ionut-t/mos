use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub dotfiles_dir: String,
}

impl GlobalConfig {
    fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| Some(dirs::home_dir()?.join(".config")))?;
        Some(base.join("mos").join("config.toml"))
    }

    pub fn load() -> Option<Self> {
        let path = Self::path()?;
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    pub fn save(dotfiles_dir: &Path) -> Result<()> {
        let path = Self::path()
            .ok_or_else(|| eyre!("could not determine config directory"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| eyre!("failed to create {}: {}", parent.display(), e))?;
        }

        let gc = GlobalConfig {
            dotfiles_dir: dotfiles_dir.display().to_string(),
        };
        let content = toml::to_string_pretty(&gc)
            .map_err(|e| eyre!("failed to serialize global config: {}", e))?;

        std::fs::write(&path, content)
            .map_err(|e| eyre!("failed to write global config to {}: {}", path.display(), e))?;

        Ok(())
    }

    pub fn dotfiles_dir(&self) -> Result<PathBuf> {
        crate::path::expand_path(&self.dotfiles_dir)
    }
}
