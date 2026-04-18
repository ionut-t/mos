pub mod brew;
pub mod cargo;
pub mod go;
pub mod script;

use crate::config::{
    Config,
    module::{BrewDep, CargoDep, Deps, GoDep, ScriptDep},
};

pub struct CollectedDeps {
    pub brew: Vec<BrewDep>,
    pub cargo: Vec<CargoDep>,
    pub go: Vec<GoDep>,
    pub script: Vec<ScriptDep>,
}

impl CollectedDeps {
    pub fn from_profile(config: &Config, profile_name: &str) -> Self {
        let profile = &config.profiles[profile_name];

        let mut collected = CollectedDeps {
            brew: vec![],
            cargo: vec![],
            go: vec![],
            script: vec![],
        };

        for module_name in &profile.modules {
            if let Some(module) = config.modules.get(module_name)
                && let Some(deps) = &module.deps
            {
                collect(&mut collected, deps);
            }
        }

        collected
    }
}

pub fn is_installed(bin: &str) -> bool {
    which::which(bin).is_ok()
}

fn collect(into: &mut CollectedDeps, deps: &Deps) {
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
