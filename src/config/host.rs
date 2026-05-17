use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Brew,
    Apt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub description: Option<String>,
    pub os: String,
    pub package_manager: Option<PackageManager>,
    pub default_profile: Option<String>,
}

pub fn detect_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn detect_os() -> String {
    std::env::consts::OS.to_string()
}
