use color_eyre::eyre::{self, Result};

pub fn is_installed(pkg: &str) -> bool {
    super::is_installed(pkg)
}

pub fn install(pkg: &str) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .arg("install")
        .arg(pkg)
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install cargo package: {}", pkg);
    }

    Ok(())
}
