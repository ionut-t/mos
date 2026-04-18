use std::path::{Path, PathBuf};

use chrono::Local;
use color_eyre::eyre::{Result, eyre};

/// Back up a file before overwriting it. Returns the path to the backup.
pub fn backup_file(file: &Path, backup_dir: &Path, format: &str) -> Result<PathBuf> {
    if !file.exists() || file.is_symlink() {
        return Err(eyre!(
            "cannot back up {}: not a regular file",
            file.display()
        ));
    }

    std::fs::create_dir_all(backup_dir).map_err(|e| {
        eyre!(
            "failed to create backup dir {}: {}",
            backup_dir.display(),
            e
        )
    })?;

    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    let datetime = Local::now().format("%d%m%Y-%H%M%S").to_string();

    let backup_name = format
        .replace("{name}", &file_name)
        .replace("{datetime}", &datetime);

    let backup_path = backup_dir.join(&backup_name);

    std::fs::copy(file, &backup_path).map_err(|e| {
        eyre!(
            "failed to back up {} to {}: {}",
            file.display(),
            backup_path.display(),
            e
        )
    })?;

    Ok(backup_path)
}
