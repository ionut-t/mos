use color_eyre::eyre::{self, Result};

pub fn is_installed(bin: &str) -> bool {
    super::is_installed(bin)
}

pub fn install(pkg: &str) -> Result<()> {
    let status = std::process::Command::new("go")
        .arg("install")
        .arg(pkg)
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install go package: {}", pkg);
    }

    Ok(())
}
