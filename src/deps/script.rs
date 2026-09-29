use std::process::{Command, Stdio};

use color_eyre::eyre::{self, Result};

use crate::config::module::ScriptDep;

pub fn is_installed(dep: &ScriptDep) -> bool {
    match &dep.check {
        Some(check) => Command::new("sh")
            .arg("-c")
            .arg(check)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success()),
        None => super::is_installed(&dep.name),
    }
}

pub fn install(name: &str, cmd: &str) -> Result<()> {
    let status = Command::new("sh").arg("-c").arg(cmd).status()?;

    if !status.success() {
        eyre::bail!("failed to install script package: {}", name);
    }

    Ok(())
}
