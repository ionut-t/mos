use color_eyre::eyre::{self, Result};

pub fn is_installed(pkg: &str) -> bool {
    super::is_installed(pkg)
}

pub fn install(name: &str, cmd: &str) -> Result<()> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()?;

    if !status.success() {
        eyre::bail!("failed to install script package: {}", name);
    }

    Ok(())
}
