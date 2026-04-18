use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use walkdir::WalkDir;

use crate::config::Config;
use crate::path::expand_path;

#[derive(Debug, Clone)]
pub struct ResolvedFile {
    pub source: PathBuf,
    pub target: PathBuf,
    pub layer: String,
}

/// Resolve files for a module using three-layer overlay: base < profile < host.
/// Higher-priority layers override lower ones for the same relative path.
pub fn resolve_files(
    config: &Config,
    module_name: &str,
    profile_name: Option<&str>,
    hostname: &str,
) -> Result<Vec<ResolvedFile>> {
    let module = &config.modules[module_name];

    let (source, target) = match (&module.source, &module.target) {
        (Some(s), Some(t)) => (s, t),
        _ => return Ok(vec![]), // deps-only module, nothing to link
    };

    let dotfiles_dir = config.dotfiles_dir()?;
    let target_base = expand_path(target)?;

    let base_source = dotfiles_dir.join("base").join(source);
    let profile_source = profile_name.map(|p| dotfiles_dir.join("profiles").join(p).join(source));
    let host_source = dotfiles_dir.join("hosts").join(hostname).join(source);

    // Check if the source is a file (not a directory)
    let is_file_mapping = base_source.is_file()
        || profile_source.as_ref().is_some_and(|p| p.is_file())
        || host_source.is_file();

    if is_file_mapping {
        return resolve_file_mapping(
            &base_source,
            profile_source.as_deref(),
            &host_source,
            &target_base,
        );
    }

    // Directory mapping: collect files from all layers
    let mut file_map: HashMap<PathBuf, ResolvedFile> = HashMap::new();

    // Layer 1: base (lowest priority)
    if base_source.is_dir() {
        collect_layer_files(&base_source, &target_base, "base", &mut file_map)?;
    }

    // Layer 2: profile
    if let Some(ref ps) = profile_source
        && ps.is_dir()
    {
        let layer_name = format!("profile/{}", profile_name.unwrap_or("unknown"));
        collect_layer_files(ps, &target_base, &layer_name, &mut file_map)?;
    }

    // Layer 3: host (highest priority)
    if host_source.is_dir() {
        let layer_name = format!("host/{}", hostname);
        collect_layer_files(&host_source, &target_base, &layer_name, &mut file_map)?;
    }

    let mut files: Vec<ResolvedFile> = file_map.into_values().collect();
    files.sort_by(|a, b| a.target.cmp(&b.target));
    Ok(files)
}

/// For file-to-file mappings, pick the highest-priority layer that has the file.
fn resolve_file_mapping(
    base_source: &Path,
    profile_source: Option<&Path>,
    host_source: &Path,
    target: &Path,
) -> Result<Vec<ResolvedFile>> {
    // Host > profile > base
    if host_source.is_file() {
        let layer = "host".to_string();
        return Ok(vec![ResolvedFile {
            source: host_source.to_path_buf(),
            target: target.to_path_buf(),
            layer,
        }]);
    }

    if let Some(ps) = profile_source
        && ps.is_file()
    {
        return Ok(vec![ResolvedFile {
            source: ps.to_path_buf(),
            target: target.to_path_buf(),
            layer: "profile".to_string(),
        }]);
    }

    if base_source.is_file() {
        return Ok(vec![ResolvedFile {
            source: base_source.to_path_buf(),
            target: target.to_path_buf(),
            layer: "base".to_string(),
        }]);
    }

    Ok(vec![])
}

fn collect_layer_files(
    source_dir: &Path,
    target_base: &Path,
    layer: &str,
    file_map: &mut HashMap<PathBuf, ResolvedFile>,
) -> Result<()> {
    for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(source_dir)
                .expect("entry should be under source_dir");
            let target_path = target_base.join(relative);

            file_map.insert(
                relative.to_path_buf(),
                ResolvedFile {
                    source: entry.path().to_path_buf(),
                    target: target_path,
                    layer: layer.to_string(),
                },
            );
        }
    }
    Ok(())
}
