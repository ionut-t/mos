use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub source: Option<String>,
    pub target: Option<String>,
    #[serde(default = "default_true")]
    pub tracked: bool,
    pub deps: Option<Deps>,
    pub files: Option<HashMap<String, FileConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deps {
    pub packages: Option<Vec<PackageDep>>,
    pub brew: Option<Vec<BrewDep>>,
    pub apt: Option<Vec<BrewDep>>,
    pub go: Option<Vec<GoDep>>,
    pub cargo: Option<Vec<CargoDep>>,
    pub script: Option<Vec<ScriptDep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageDep {
    Simple(String),
    Platform {
        brew: Option<String>,
        apt: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BrewDep {
    Simple(String),
    Named { pkg: String, bin: String },
}

impl BrewDep {
    pub fn pkg(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Named { pkg, .. } => pkg,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GoDep {
    Simple(String),
    Named { pkg: String, bin: String },
}

impl GoDep {
    pub fn pkg(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Named { pkg, .. } => pkg,
        }
    }
    pub fn bin(&self) -> &str {
        match self {
            Self::Simple(s) => {
                // Strip @version suffix first, then take the last path segment
                // Handle versioned paths like github.com/user/repo/v2@latest → repo
                let without_version = s.split('@').next().unwrap_or(s);
                let last = without_version
                    .split('/')
                    .next_back()
                    .unwrap_or(without_version);
                // If last segment looks like a major version (v2, v3...), take the one before it
                if last.starts_with('v') && last[1..].parse::<u32>().is_ok() {
                    without_version.split('/').rev().nth(1).unwrap_or(last)
                } else {
                    last
                }
            }
            Self::Named { bin, .. } => bin,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CargoDep {
    Simple(String),
    Named { pkg: String, bin: String },
}

impl CargoDep {
    pub fn pkg(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Named { pkg, .. } => pkg,
        }
    }
    pub fn bin(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Named { bin, .. } => bin,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDep {
    pub name: String,
    pub cmd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_true")]
    pub tracked: bool,
}

fn default_true() -> bool {
    true
}
