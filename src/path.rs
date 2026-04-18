use std::env;
use std::path::PathBuf;

use color_eyre::eyre::{Result, eyre};

/// Expand `~` to home directory and `$VAR`/`${VAR}` to environment variable values.
pub fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = expand_tilde(path)?;
    let expanded = expand_env_vars(&expanded)?;
    Ok(PathBuf::from(expanded))
}

fn expand_tilde(path: &str) -> Result<String> {
    if path == "~" {
        let home = dirs::home_dir().ok_or_else(|| eyre!("could not determine home directory"))?;
        Ok(home.to_string_lossy().into_owned())
    } else if let Some(rest) = path.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or_else(|| eyre!("could not determine home directory"))?;
        Ok(format!("{}/{}", home.display(), rest))
    } else {
        Ok(path.to_string())
    }
}

fn expand_env_vars(input: &str) -> Result<String> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            let var_name = if chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let name: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
                name
            } else {
                let name: String = chars
                    .by_ref()
                    .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                    .collect();
                name
            };

            if var_name.is_empty() {
                result.push('$');
            } else {
                let value = env::var(&var_name)
                    .map_err(|_| eyre!("environment variable '{}' not set", var_name))?;
                result.push_str(&value);
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}
