use std::collections::HashSet;

use color_eyre::eyre::{self, Result};

pub struct InstalledCache {
    packages: HashSet<String>,
}

impl InstalledCache {
    pub fn load() -> Self {
        let mut packages = HashSet::new();

        if let Ok(out) = std::process::Command::new("apt")
            .args(["list", "--installed"])
            .stderr(std::process::Stdio::null())
            .output()
            && out.status.success()
        {
            String::from_utf8_lossy(&out.stdout).lines().for_each(|l| {
                if let Some(name) = l.split('/').next() {
                    packages.insert(name.trim().to_string());
                }
            });
        }

        Self { packages }
    }

    pub fn is_installed(&self, pkg: &str) -> bool {
        self.packages.contains(pkg)
    }
}

pub fn install(pkgs: &[&str]) -> Result<()> {
    let status = std::process::Command::new("sudo")
        .args(["apt-get", "install", "-y"])
        .args(pkgs)
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install apt packages: {}", pkgs.join(", "));
    }

    Ok(())
}
