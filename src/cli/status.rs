use std::path::Path;

use crate::{
    config::{
        Config,
        host::{detect_hostname, detect_os},
    },
    state::{self, State, diff::check_all_drift},
    ui,
};
use color_eyre::eyre::Result;
use colored::Colorize;

#[derive(clap::Parser, Debug)]
pub struct StatusCommand;

impl StatusCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let state = State::load()?;
        let hostname = detect_hostname();
        let os = detect_os();

        println!("{} {}", "Hostname:".bold(), hostname);
        println!("{} {}", "OS:      ".bold(), os);
        println!(
            "{} {}",
            "Profile: ".bold(),
            state.active_profile.as_deref().unwrap_or("(none)")
        );
        if let Some(ref sync) = state.last_sync {
            println!("{} {}", "Last sync:".bold(), sync);
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
            ui::warn("No modules configured.");
            return Ok(());
        }

        // Drift detection
        let drift_reports = check_all_drift(&state.links);

        let name_width = profile_modules.iter().map(|m| m.len()).max().unwrap_or(0);

        for module_name in &profile_modules {
            let linked = state.links.contains_key(module_name);
            let padded_name = format!("{:<width$}", module_name, width = name_width);

            if linked {
                let link_state = &state.links[module_name];
                let drift = drift_reports.iter().find(|r| r.module == *module_name);
                let has_drift = drift.is_some_and(|d| d.has_drift());
                let drift_status = if has_drift {
                    format!(" {}", "[DRIFT DETECTED]".red().bold())
                } else {
                    String::new()
                };
                let file_count = link_state.files.len();
                let file_word = if file_count == 1 { "file" } else { "files" };
                println!(
                    "  {} {}  {}  {} {}{}",
                    ui::bullet(true),
                    padded_name.bold(),
                    layer_tag(&link_state.source_layer),
                    file_count,
                    file_word.dimmed(),
                    drift_status,
                );

                // Show individual file drift
                if let Some(d) = drift {
                    for (path, status) in &d.files {
                        if *status != state::diff::FileStatus::Ok {
                            println!(
                                "    {} {} - {}",
                                "⚠".yellow(),
                                path,
                                status.to_string().yellow()
                            );
                        }
                    }
                }
            } else {
                let deps_only = config
                    .modules
                    .get(module_name)
                    .is_some_and(|m| m.source.is_none());
                let note = if deps_only { " (deps only)" } else { "" };
                println!(
                    "  {} {}",
                    ui::bullet(false),
                    format!("{}{}", padded_name, note).dimmed()
                );
            }
        }

        Ok(())
    }
}

/// A colored `[layer]` tag: base is neutral, profile/host overrides are highlighted
/// since they mean "this file diverges from the shared base config."
fn layer_tag(layer: &str) -> String {
    let tag = format!("[{}]", layer);
    if layer.starts_with("host/") {
        tag.magenta().to_string()
    } else if layer.starts_with("profile/") {
        tag.cyan().to_string()
    } else {
        tag.dimmed().to_string()
    }
}
