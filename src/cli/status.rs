use std::path::Path;

use crate::{
    config::{
        Config,
        host::{detect_hostname, detect_os},
    },
    state::{self, State, diff::check_all_drift},
};
use color_eyre::eyre::Result;

#[derive(clap::Parser, Debug)]
pub struct StatusCommand;

impl StatusCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let state = State::load()?;
        let hostname = detect_hostname();
        let os = detect_os();

        println!("Hostname: {}", hostname);
        println!("OS:       {}", os);
        println!(
            "Profile:  {}",
            state.active_profile.as_deref().unwrap_or("(none)")
        );
        if let Some(ref sync) = state.last_sync {
            println!("Last sync: {}", sync);
        }
        println!();

        // Determine modules to show
        let profile_modules: Vec<String> = if let Some(ref profile_name) = state.active_profile {
            config
                .profiles
                .get(profile_name)
                .map(|p| p.modules.clone())
                .unwrap_or_default()
        } else {
            config.modules.keys().cloned().collect()
        };

        if profile_modules.is_empty() {
            println!("No modules configured.");
            return Ok(());
        }

        // Drift detection
        let drift_reports = check_all_drift(&state.links);

        for module_name in &profile_modules {
            let linked = state.links.contains_key(module_name);
            if linked {
                let link_state = &state.links[module_name];
                let drift = drift_reports.iter().find(|r| r.module == *module_name);
                let drift_status = if drift.is_some_and(|d| d.has_drift()) {
                    " [DRIFT DETECTED]"
                } else {
                    ""
                };
                println!(
                    "  {} [linked] layer={} files={}{}",
                    module_name,
                    link_state.source_layer,
                    link_state.files.len(),
                    drift_status,
                );

                // Show individual file drift
                if let Some(d) = drift {
                    for (path, status) in &d.files {
                        if *status != state::diff::FileStatus::Ok {
                            println!("    {} - {}", path, status);
                        }
                    }
                }
            } else {
                println!("  {} [unlinked]", module_name);
            }
        }

        Ok(())
    }
}
