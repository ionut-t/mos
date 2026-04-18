use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub tracked: bool,
    #[serde(default)]
    pub modules: Vec<String>,
}

fn default_true() -> bool {
    true
}
