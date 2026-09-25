pub mod backup;
pub mod overlay;

use std::path::Path;

use chrono::Local;
use color_eyre::eyre::{Result, eyre};
use colored::Colorize;

use crate::config::Config;
use crate::state::{LinkState, LinkedFile, State};
use crate::ui;

use self::backup::backup_file;
use self::overlay::resolve_files;

/// Link a module: resolve overlay files, create symlinks, update state.
pub fn link_module(
    config: &Config,
    state: &mut State,
    module_name: &str,
    profile: Option<&str>,
    hostname: &str,
) -> Result<()> {
    let module = config
        .modules
        .get(module_name)
        .ok_or_else(|| eyre!("module '{}' not found in config", module_name))?;

    // deps-only module: nothing to link, nothing to report.
    if module.source.is_none() {
        return Ok(());
    }

    let resolved = resolve_files(config, module_name, profile, hostname)?;

    if resolved.is_empty() {
        println!(
            "{} {} — no files found for configured source",
            "⚠".yellow(),
            module_name
        );
        return Ok(());
    }

    let backup_dir = config.backup_dir()?;
    let backup_format = &config.settings.backup_format;

    let mut linked_files = Vec::new();
    let mut primary_layer = String::new();
    let mut backed_up = 0usize;
    let mut created = 0usize;

    for file in &resolved {
        // Track the highest-priority layer seen
        primary_layer = file.layer.clone();

        if file.target.is_symlink() {
            let current = std::fs::read_link(&file.target)?;
            if current == file.source {
                // Already correctly linked
                linked_files.push(LinkedFile {
                    source: file.source.clone(),
                    target: file.target.clone(),
                });
                continue;
            }
            // Wrong target — remove and re-create
            std::fs::remove_file(&file.target)
                .map_err(|e| eyre!("failed to remove symlink {}: {}", file.target.display(), e))?;
        } else if file.target.exists() {
            // Regular file — back it up first
            let backup_path = backup_file(&file.target, &backup_dir, backup_format)?;
            println!(
                "  {} Backed up {} -> {}",
                "↺".yellow(),
                file.target.display(),
                backup_path.display()
            );
            state.backups.insert(
                file.target.display().to_string(),
                backup_path.display().to_string(),
            );
            std::fs::remove_file(&file.target)
                .map_err(|e| eyre!("failed to remove {}: {}", file.target.display(), e))?;
            backed_up += 1;
        }

        // Create parent directories if needed
        if let Some(parent) = file.target.parent() {
            ensure_parent_dir(parent)?;
        }

        // Create symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file.source, &file.target).map_err(|e| {
            eyre!(
                "failed to create symlink {} -> {}: {}",
                file.target.display(),
                file.source.display(),
                e
            )
        })?;

        #[cfg(not(unix))]
        return Err(eyre!("symlink creation is only supported on Unix"));

        created += 1;
        linked_files.push(LinkedFile {
            source: file.source.clone(),
            target: file.target.clone(),
        });
    }

    let file_word = if resolved.len() == 1 { "file" } else { "files" };
    let mut detail = format!("{} {} [{}]", resolved.len(), file_word, primary_layer);
    if created > 0 {
        detail.push_str(&format!(", {} linked", created));
    }
    if backed_up > 0 {
        detail.push_str(&format!(", {} backed up", backed_up));
    }
    println!("{} {} {}", "✓".green(), module_name.bold(), detail.dimmed());

    state.links.insert(
        module_name.to_string(),
        LinkState {
            target: module.target.clone().unwrap_or_default(),
            source_layer: primary_layer,
            files: linked_files,
            linked_at: Local::now().to_rfc3339(),
        },
    );

    Ok(())
}

/// Ensure a directory exists, removing any broken symlinks in the ancestor chain
/// that would prevent `create_dir_all` from succeeding.
fn ensure_parent_dir(path: &Path) -> Result<()> {
    // Collect ancestors from root down to path (exclusive)
    let mut ancestors: Vec<&Path> = path.ancestors().collect();
    ancestors.reverse();

    for ancestor in &ancestors {
        if ancestor.is_symlink() && !ancestor.exists() {
            std::fs::remove_file(ancestor).map_err(|e| {
                eyre!(
                    "failed to remove broken symlink {}: {}",
                    ancestor.display(),
                    e
                )
            })?;
        }
    }

    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| eyre!("failed to create directory {}: {}", path.display(), e))?;
    }

    Ok(())
}

/// Unlink a module: remove symlinks, update state.
pub fn unlink_module(state: &mut State, module_name: &str) -> Result<()> {
    let link_state = state
        .links
        .get(module_name)
        .ok_or_else(|| eyre!("module '{}' is not currently linked", module_name))?;

    for linked_file in &link_state.files {
        let target = &linked_file.target;
        if target.is_symlink() {
            if let Ok(actual) = std::fs::read_link(target) {
                if actual == linked_file.source {
                    std::fs::remove_file(target).map_err(|e| {
                        eyre!("failed to remove symlink {}: {}", target.display(), e)
                    })?;
                    println!("  {} Removed {}", "✓".green(), target.display());
                } else {
                    ui::item(format!(
                        "{} Skipped {} (points to different source)",
                        "⚠".yellow(),
                        target.display()
                    ));
                }
            }
        } else if target.exists() {
            ui::item(format!(
                "{} Skipped {} (not a symlink, may have been modified)",
                "⚠".yellow(),
                target.display()
            ));
        } else {
            ui::detail(format!("Skipped {} (already gone)", target.display()));
        }
    }

    state.links.remove(module_name);
    Ok(())
}
