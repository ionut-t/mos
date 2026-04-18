use std::fmt;

use super::LinkState;

#[derive(Debug, PartialEq, Eq)]
pub enum FileStatus {
    Ok,
    Modified,
    Missing,
    WrongTarget,
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStatus::Ok => write!(f, "ok"),
            FileStatus::Modified => write!(f, "modified"),
            FileStatus::Missing => write!(f, "missing"),
            FileStatus::WrongTarget => write!(f, "wrong target"),
        }
    }
}

pub struct DriftReport {
    pub module: String,
    pub files: Vec<(String, FileStatus)>,
}

impl DriftReport {
    pub fn has_drift(&self) -> bool {
        self.files.iter().any(|(_, s)| *s != FileStatus::Ok)
    }
}

pub fn check_drift(module_name: &str, link_state: &LinkState) -> DriftReport {
    let mut files = Vec::new();

    for linked_file in &link_state.files {
        let target = &linked_file.target;
        let expected_source = &linked_file.source;

        let status = if !target.exists() && !target.is_symlink() {
            FileStatus::Missing
        } else if target.is_symlink() {
            match std::fs::read_link(target) {
                Ok(actual) => {
                    if actual == *expected_source {
                        FileStatus::Ok
                    } else {
                        FileStatus::WrongTarget
                    }
                }
                Err(_) => FileStatus::Missing,
            }
        } else {
            // Target exists but is not a symlink — it was replaced
            FileStatus::Modified
        };

        files.push((target.display().to_string(), status));
    }

    DriftReport {
        module: module_name.to_string(),
        files,
    }
}

pub fn check_all_drift(links: &std::collections::HashMap<String, LinkState>) -> Vec<DriftReport> {
    links
        .iter()
        .map(|(name, state)| check_drift(name, state))
        .collect()
}
