use color_eyre::eyre::{Result, eyre};

use crate::{
    config::{Config, host},
    deps::{self, CollectedDeps},
    state::State,
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

fn check(config_path: &std::path::Path) -> Result<()> {
    let (collected, profile_name) = collect(config_path)?;

    println!("Checking dependencies for profile '{}'...\n", profile_name);

    let apt_cache = deps::apt::InstalledCache::load();
    let brew_cache = deps::brew::InstalledCache::load();
    let mut all_ok = true;

    for dep in &collected.apt {
        let ok = apt_cache.is_installed(dep.pkg());
        println!(
            "  [apt] {} — {}",
            dep.pkg(),
            if ok { "ok" } else { "missing" }
        );
        if !ok {
            all_ok = false;
        }
    }

    for dep in &collected.brew {
        let ok = brew_cache.is_installed(dep.pkg());
        println!(
            "  [brew] {} — {}",
            dep.pkg(),
            if ok { "ok" } else { "missing" }
        );
        if !ok {
            all_ok = false;
        }
    }

    for dep in &collected.script {
        let ok = deps::script::is_installed(&dep.name);
        println!(
            "  [script] {} — {}",
            dep.name,
            if ok { "ok" } else { "missing" }
        );
        if !ok {
            all_ok = false;
        }
    }

    for dep in &collected.cargo {
        let ok = deps::cargo::is_installed(dep.bin());
        println!(
            "  [cargo] {} — {}",
            dep.pkg(),
            if ok { "ok" } else { "missing" }
        );
        if !ok {
            all_ok = false;
        }
    }

    for dep in &collected.go {
        let ok = deps::go::is_installed(dep.bin());
        println!(
            "  [go] {} — {}",
            dep.pkg(),
            if ok { "ok" } else { "missing" }
        );
        if !ok {
            all_ok = false;
        }
    }

    if all_ok {
        println!("\nAll dependencies satisfied.");
    } else {
        println!("\nRun `mos deps install` to install missing dependencies.");
    }

    Ok(())
}

fn install(config_path: &std::path::Path) -> Result<()> {
    let (collected, profile_name) = collect(config_path)?;

    println!(
        "Installing missing dependencies for profile '{}'...\n",
        profile_name
    );

    let apt_cache = deps::apt::InstalledCache::load();
    let brew_cache = deps::brew::InstalledCache::load();
    let mut failed: Vec<String> = vec![];

    for dep in &collected.apt {
        if !apt_cache.is_installed(dep.pkg()) {
            println!("  [apt] installing {}...", dep.pkg());
            if let Err(e) = deps::apt::install(dep.pkg()) {
                eprintln!("  failed: {}", e);
                failed.push(dep.pkg().to_string());
            }
        }
    }

    let missing_brew: Vec<&str> = collected
        .brew
        .iter()
        .filter(|dep| !brew_cache.is_installed(dep.pkg()))
        .map(|dep| dep.pkg())
        .collect();

    if !missing_brew.is_empty() {
        println!("  [brew] installing {}...", missing_brew.join(", "));
        if let Err(e) = deps::brew::install(&missing_brew) {
            eprintln!("  failed: {}", e);
            failed.extend(missing_brew.into_iter().map(str::to_string));
        }
    }
    for dep in &collected.script {
        if !deps::script::is_installed(&dep.name) {
            println!("  [script] running {}...", dep.name);
            if let Err(e) = deps::script::install(&dep.name, &dep.cmd) {
                eprintln!("  failed: {}", e);
                failed.push(dep.name.clone());
            }
        }
    }
    for dep in &collected.cargo {
        if !deps::cargo::is_installed(dep.bin()) {
            println!("  [cargo] installing {}...", dep.pkg());
            if let Err(e) = deps::cargo::install(dep.pkg()) {
                eprintln!("  failed: {}", e);
                failed.push(dep.pkg().to_string());
            }
        }
    }
    for dep in &collected.go {
        if !deps::go::is_installed(dep.bin()) {
            println!("  [go] installing {}...", dep.pkg());
            if let Err(e) = deps::go::install(dep.pkg()) {
                eprintln!("  failed: {}", e);
                failed.push(dep.pkg().to_string());
            }
        }
    }

    if failed.is_empty() {
        println!("\nDone.");
    } else {
        println!(
            "\nDone with errors. Failed to install: {}",
            failed.join(", ")
        );
    }

    Ok(())
}
