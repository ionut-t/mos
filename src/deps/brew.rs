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

/// Taps Homebrew has been told to trust (`brew trust --tap`).
fn trusted_taps() -> HashSet<String> {
    #[derive(serde::Deserialize)]
    struct Trust {
        #[serde(default)]
        taps: Vec<String>,
    }

    std::process::Command::new("brew")
        .args(["trust", "--json", "v1"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| serde_json::from_slice::<Trust>(&out.stdout).ok())
        .map(|t| t.taps.into_iter().collect())
        .unwrap_or_default()
}

/// Taps and trusts each tap that isn't trusted yet. Homebrew 7+ refuses to
/// load formulae or casks from untrusted third-party taps.
pub fn trust_taps(taps: &[&str]) -> Result<()> {
    let trusted = trusted_taps();

    for tap in taps.iter().filter(|t| !trusted.contains(**t)) {
        for args in [vec!["tap", tap], vec!["trust", "--tap", tap]] {
            let status = std::process::Command::new("brew").args(&args).status()?;
            if !status.success() {
                eyre::bail!("`brew {}` failed", args.join(" "));
            }
        }
    }

    Ok(())
}

pub fn install(pkgs: &[&str]) -> Result<()> {
    let status = std::process::Command::new("brew")
        .arg("install")
        .args(pkgs)
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install brew packages: {}", pkgs.join(", "));
    }

    Ok(())
}
