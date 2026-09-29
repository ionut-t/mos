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
    Named {
        pkg: String,
        bin: Option<String>,
        /// Third-party tap the package comes from, e.g. `felixkratz/formulae`.
        /// Homebrew refuses to load formulae from taps that aren't trusted,
        /// so mos taps and trusts it before installing.
        tap: Option<String>,
    },
}

impl BrewDep {
    pub fn pkg(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Named { pkg, .. } => pkg,
        }
    }

    /// Name as reported by `brew list` / `apt list`: the last segment of a
    /// fully-qualified name like `felixkratz/formulae/borders`.
    pub fn short_name(&self) -> &str {
        let pkg = self.pkg();
        pkg.rsplit('/').next().unwrap_or(pkg)
    }

    /// Tap to trust before installing: the explicit `tap` field, or the
    /// `user/repo` prefix of a fully-qualified name.
    pub fn tap(&self) -> Option<&str> {
        if let Self::Named { tap: Some(tap), .. } = self {
            return Some(tap);
        }
        let pkg = self.pkg();
        let (tap, _) = pkg.rsplit_once('/')?;
        tap.contains('/').then_some(tap)
    }

    /// Name to pass to `brew install`: fully-qualified when a tap is known,
    /// so Homebrew can't resolve it to a same-named formula elsewhere.
    pub fn install_name(&self) -> String {
        match self.tap() {
            Some(tap) => format!("{}/{}", tap, self.short_name()),
            None => self.pkg().to_string(),
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
    /// Shell command that exits 0 when the dependency is installed.
    /// Defaults to checking `name` is on `PATH`.
    pub check: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_true")]
    pub tracked: bool,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(pkg: &str, tap: Option<&str>) -> BrewDep {
        BrewDep::Named {
            pkg: pkg.into(),
            bin: None,
            tap: tap.map(Into::into),
        }
    }

    #[test]
    fn brew_dep_plain_name() {
        let dep = BrewDep::Simple("ripgrep".into());
        assert_eq!(dep.short_name(), "ripgrep");
        assert_eq!(dep.tap(), None);
        assert_eq!(dep.install_name(), "ripgrep");
    }

    #[test]
    fn brew_dep_explicit_tap() {
        let dep = named("borders", Some("felixkratz/formulae"));
        assert_eq!(dep.short_name(), "borders");
        assert_eq!(dep.tap(), Some("felixkratz/formulae"));
        assert_eq!(dep.install_name(), "felixkratz/formulae/borders");
    }

    #[test]
    fn brew_dep_fully_qualified_name() {
        let dep = BrewDep::Simple("felixkratz/formulae/borders".into());
        assert_eq!(dep.short_name(), "borders");
        assert_eq!(dep.tap(), Some("felixkratz/formulae"));
        assert_eq!(dep.install_name(), "felixkratz/formulae/borders");
    }

    #[test]
    fn brew_dep_parses_from_toml() {
        #[derive(Deserialize)]
        struct W {
            brew: Vec<BrewDep>,
        }
        let w: W = toml::from_str(
            r#"brew = ["git", { pkg = "fd-find", bin = "fdfind" }, { pkg = "borders", tap = "felixkratz/formulae" }]"#,
        )
        .unwrap();
        assert_eq!(w.brew[1].tap(), None);
        assert_eq!(w.brew[2].install_name(), "felixkratz/formulae/borders");
    }
}
