pub mod apt;
pub mod brew;
pub mod cargo;
pub mod go;
pub mod script;

use std::collections::HashSet;

use crate::config::{
    Config,
    host::PackageManager,
    module::{BrewDep, CargoDep, Deps, GoDep, PackageDep, ScriptDep},
};

pub struct CollectedDeps {
    pub apt: Vec<BrewDep>,
    pub brew: Vec<BrewDep>,
    pub cargo: Vec<CargoDep>,
    pub go: Vec<GoDep>,
    pub script: Vec<ScriptDep>,
}

impl CollectedDeps {
    pub fn from_profile(
        config: &Config,
        profile_name: &str,
        pkg_manager: Option<&PackageManager>,
    ) -> Self {
        let profile = &config.profiles[profile_name];

        let mut collected = CollectedDeps {
            apt: vec![],
            brew: vec![],
            cargo: vec![],
            go: vec![],
            script: vec![],
        };

        for module_name in &profile.modules {
            if let Some(module) = config.modules.get(module_name)
                && let Some(deps) = &module.deps
            {
                collect(&mut collected, deps, pkg_manager);
            }
        }

        collected.apt = dedup(collected.apt);
        collected.brew = dedup(collected.brew);
        collected
    }
}

fn dedup(deps: Vec<BrewDep>) -> Vec<BrewDep> {
    let mut seen = HashSet::new();
    deps.into_iter()
        .filter(|d| seen.insert(d.pkg().to_string()))
        .collect()
}

pub fn is_installed(bin: &str) -> bool {
    which::which(bin).is_ok()
}

fn collect(into: &mut CollectedDeps, deps: &Deps, pkg_manager: Option<&PackageManager>) {
    if let Some(pkgs) = &deps.packages {
        let use_brew = matches!(pkg_manager, Some(PackageManager::Brew) | None)
            && (pkg_manager.is_some() || is_installed("brew"));
        let use_apt = matches!(pkg_manager, Some(PackageManager::Apt) | None)
            && (pkg_manager.is_some() || is_installed("apt"));

        for dep in pkgs {
            match dep {
                PackageDep::Simple(name) => {
                    if use_brew {
                        into.brew.push(BrewDep::Simple(name.clone()));
                    }
                    if use_apt {
                        into.apt.push(BrewDep::Simple(name.clone()));
                    }
                }
                PackageDep::Platform { brew, apt } => {
                    if use_brew && let Some(name) = brew {
                        into.brew.push(BrewDep::Simple(name.clone()));
                    }

                    if use_apt && let Some(name) = apt {
                        into.apt.push(BrewDep::Simple(name.clone()));
                    }
                }
            }
        }
    }
    if let Some(pkgs) = &deps.apt {
        into.apt.extend(pkgs.iter().cloned());
    }
    if let Some(pkgs) = &deps.brew {
        into.brew.extend(pkgs.iter().cloned());
    }
    if let Some(pkgs) = &deps.cargo {
        into.cargo.extend(pkgs.iter().cloned());
    }
    if let Some(pkgs) = &deps.go {
        into.go.extend(pkgs.iter().cloned());
    }
    if let Some(scripts) = &deps.script {
        into.script.extend(scripts.iter().cloned());
    }
}
