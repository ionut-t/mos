use color_eyre::eyre::{Result, eyre};
use colored::Colorize;

use crate::{
    config::{
        Config, host,
        module::{BrewDep, ScriptDep},
    },
    deps::{self, CollectedDeps},
    state::State,
    ui,
};

#[derive(clap::Parser, Clone)]
pub struct DepsCommand {
    #[clap(subcommand)]
    command: DepsCommands,
}

#[derive(clap::Subcommand, Clone)]
enum DepsCommands {
    /// Check if dependencies are installed for the active profile
    Check,
    /// Install missing dependencies for the active profile
    Install,
}

impl DepsCommand {
    pub fn run(&self, config_path: &std::path::Path) -> Result<()> {
        match &self.command {
            DepsCommands::Check => check(config_path),
            DepsCommands::Install => install(config_path),
        }
    }
}

fn collect(config_path: &std::path::Path) -> Result<(CollectedDeps, String)> {
    let config = Config::load(config_path)?;
    let state = State::load()?;

    let profile_name = state
        .active_profile
        .as_deref()
        .ok_or_else(|| eyre!("no active profile set - run `mos profile switch <name>` first"))?
        .to_string();

    if !config.profiles.contains_key(&profile_name) {
        return Err(eyre!(
            "active profile '{}' does not exist in config",
            profile_name
        ));
    }

    let pkg_manager = config
        .hosts
        .get(&host::detect_hostname())
        .and_then(|h| h.package_manager.as_ref());

    let collected = CollectedDeps::from_profile(&config, &profile_name, pkg_manager);

    Ok((collected, profile_name))
}

fn run_step(
    idx: &mut usize,
    total: usize,
    tag: &str,
    pkg: &str,
    install: impl FnOnce() -> Result<()>,
    failed: &mut Vec<String>,
) {
    *idx += 1;
    println!("[{}/{}] [{}] installing {}...", idx, total, tag, pkg);
    match install() {
        Ok(()) => ui::success(format!("[{}] {}", tag, pkg)),
        Err(e) => {
            ui::error(format!("[{}] {}: {}", tag, pkg, e));
            failed.push(pkg.to_string());
        }
    }
}

fn check_line(tag: &str, name: &str, ok: bool) {
    if ok {
        println!("  {} [{}] {}", "✓".green(), tag, name);
    } else {
        println!("  {} [{}] {} {}", "✗".red(), tag, name, "missing".red());
    }
}

fn check(config_path: &std::path::Path) -> Result<()> {
    let (collected, profile_name) = collect(config_path)?;

    ui::step(format!(
        "Checking dependencies for profile '{}'...",
        profile_name
    ));
    println!();

    let apt_cache = deps::apt::InstalledCache::load();
    let brew_cache = deps::brew::InstalledCache::load();
    let mut all_ok = true;

    for dep in &collected.apt {
        let ok = apt_cache.is_installed(dep.short_name());
        check_line("apt", dep.pkg(), ok);
        all_ok &= ok;
    }

    for dep in &collected.brew {
        let ok = brew_cache.is_installed(dep.short_name());
        check_line("brew", dep.pkg(), ok);
        all_ok &= ok;
    }

    for dep in &collected.script {
        let ok = deps::script::is_installed(dep);
        check_line("script", &dep.name, ok);
        all_ok &= ok;
    }

    for dep in &collected.cargo {
        let ok = deps::cargo::is_installed(dep.bin());
        check_line("cargo", dep.pkg(), ok);
        all_ok &= ok;
    }

    for dep in &collected.go {
        let ok = deps::go::is_installed(dep.bin());
        check_line("go", dep.pkg(), ok);
        all_ok &= ok;
    }

    println!();
    if all_ok {
        ui::success("All dependencies satisfied.");
    } else {
        ui::warn("Run `mos deps install` to install missing dependencies.");
    }

    Ok(())
}

fn install(config_path: &std::path::Path) -> Result<()> {
    let (collected, profile_name) = collect(config_path)?;

    let apt_cache = deps::apt::InstalledCache::load();
    let brew_cache = deps::brew::InstalledCache::load();

    let missing_apt: Vec<&str> = collected
        .apt
        .iter()
        .filter(|dep| !apt_cache.is_installed(dep.short_name()))
        .map(|dep| dep.pkg())
        .collect();
    let missing_brew: Vec<&BrewDep> = collected
        .brew
        .iter()
        .filter(|dep| !brew_cache.is_installed(dep.short_name()))
        .collect();
    let mut brew_taps: Vec<&str> = missing_brew.iter().filter_map(|dep| dep.tap()).collect();
    brew_taps.sort_unstable();
    brew_taps.dedup();
    let missing_script: Vec<&ScriptDep> = collected
        .script
        .iter()
        .filter(|dep| !deps::script::is_installed(dep))
        .collect();
    let missing_cargo: Vec<&str> = collected
        .cargo
        .iter()
        .filter(|dep| !deps::cargo::is_installed(dep.bin()))
        .map(|dep| dep.pkg())
        .collect();
    let missing_go: Vec<&str> = collected
        .go
        .iter()
        .filter(|dep| !deps::go::is_installed(dep.bin()))
        .map(|dep| dep.pkg())
        .collect();

    let total = missing_apt.len()
        + usize::from(!brew_taps.is_empty())
        + usize::from(!missing_brew.is_empty())
        + missing_script.len()
        + missing_cargo.len()
        + missing_go.len();

    if total == 0 {
        ui::success(format!(
            "All dependencies for profile '{}' are already installed.",
            profile_name
        ));
        return Ok(());
    }

    ui::step(format!(
        "Installing missing dependencies for profile '{}'...",
        profile_name
    ));

    // These installers shell out to brew/apt/cargo/go/sh with the terminal
    // inherited — they can prompt for a sudo password or print their own
    // multi-line progress (brew casks, cargo builds). A live progress bar
    // would fight them for the terminal, so each step gets a plain
    // announce-then-result pair instead of a redrawing widget.
    let mut idx = 0usize;
    let mut failed: Vec<String> = vec![];

    for pkg in &missing_apt {
        run_step(
            &mut idx,
            total,
            "apt",
            pkg,
            || deps::apt::install(pkg),
            &mut failed,
        );
    }

    if !brew_taps.is_empty() {
        // A failed trust isn't fatal: packages from other taps can still
        // install, and the affected ones fail with brew's own error below.
        run_step(
            &mut idx,
            total,
            "brew",
            &format!("trusted taps {}", brew_taps.join(", ")),
            || deps::brew::trust_taps(&brew_taps),
            &mut failed,
        );
    }

    if !missing_brew.is_empty() {
        let names: Vec<String> = missing_brew.iter().map(|dep| dep.install_name()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut batch_failed = vec![];
        run_step(
            &mut idx,
            total,
            "brew",
            &names.join(", "),
            || deps::brew::install(&names),
            &mut batch_failed,
        );

        // brew aborts the whole batch when one package fails to resolve, so
        // retry individually to install the rest and pinpoint the culprit.
        if !batch_failed.is_empty() && names.len() > 1 {
            ui::warn("brew batch failed - retrying packages individually");
            let brew_cache = deps::brew::InstalledCache::load();
            for (dep, name) in missing_brew.iter().zip(&names) {
                if brew_cache.is_installed(dep.short_name()) {
                    continue;
                }
                match deps::brew::install(&[name]) {
                    Ok(()) => ui::success(format!("[brew] {}", name)),
                    Err(e) => {
                        ui::error(format!("[brew] {}: {}", name, e));
                        failed.push(name.to_string());
                    }
                }
            }
        } else {
            failed.extend(batch_failed);
        }
    }

    for dep in &missing_script {
        run_step(
            &mut idx,
            total,
            "script",
            &dep.name,
            || deps::script::install(&dep.name, &dep.cmd),
            &mut failed,
        );
    }

    for pkg in &missing_cargo {
        run_step(
            &mut idx,
            total,
            "cargo",
            pkg,
            || deps::cargo::install(pkg),
            &mut failed,
        );
    }

    for pkg in &missing_go {
        run_step(
            &mut idx,
            total,
            "go",
            pkg,
            || deps::go::install(pkg),
            &mut failed,
        );
    }

    println!();
    if failed.is_empty() {
        ui::success("Done.");
    } else {
        ui::error(format!(
            "Done with errors. Failed to install: {}",
            failed.join(", ")
        ));
    }

    Ok(())
}
