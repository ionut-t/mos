use std::collections::HashSet;

use color_eyre::eyre::{self, Result};

pub struct InstalledCache {
    packages: HashSet<String>,
}

impl InstalledCache {
    pub fn load() -> Self {
        let mut packages = HashSet::new();

        for flag in ["--formula", "--cask"] {
            if let Ok(out) = std::process::Command::new("brew")
                .args(["list", flag])
                .output()
                && out.status.success()
            {
                String::from_utf8_lossy(&out.stdout).lines().for_each(|l| {
                    packages.insert(l.trim().to_string());
                });
            }
        }

        Self { packages }
    }

    pub fn is_installed(&self, pkg: &str) -> bool {
        self.packages.contains(pkg)
    }
}

pub fn install(pkg: &str) -> Result<()> {
    let status = std::process::Command::new("brew")
        .args(["install", pkg])
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install brew package: {}", pkg);
    }

    Ok(())
}
