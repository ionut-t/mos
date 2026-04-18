use std::path::Path;

use color_eyre::eyre::{Result, eyre};

use crate::{
    config::{
        Config,
        host::{detect_hostname, detect_os},
    },
    linker,
    state::State,
};

#[derive(clap::Parser, Debug)]
pub struct LinkCommand {
    /// Module name to link (omit to link all modules in the active profile)
    module: Option<String>,
    /// Override the active profile
    #[arg(long)]
    profile: Option<String>,
}

impl LinkCommand {
    pub fn run(&self, config_path: &Path) -> Result<()> {
        let config = Config::load(config_path)?;
        let mut state = State::load()?;
        let hostname = detect_hostname();
        let os = detect_os();

        state.hostname = hostname.clone();
        state.os = os;

        // Determine active profile
        let profile = resolve_profile(&config, &state, &hostname, self.profile.as_deref())?;

        if let Some(ref p) = profile
            && state.active_profile.as_deref() != Some(p)
        {
            state.active_profile = Some(p.clone());
        }

        if self.module.is_none() {
            // Link all modules in the active profile
            let profile_name = profile.as_deref().ok_or_else(|| {
            eyre!("no active profile; use --profile to specify one, or set a default_profile on your host")
        })?;

            let profile_config = config
                .profiles
                .get(profile_name)
                .ok_or_else(|| eyre!("profile '{}' not found in config", profile_name))?;

            println!("Linking all modules in profile '{}'...", profile_name);
            for module_name in &profile_config.modules {
                println!("Module '{}':", module_name);
                linker::link_module(
                    &config,
                    &mut state,
                    module_name,
                    Some(profile_name),
                    &hostname,
                )?;
            }
        } else if let Some(ref module_name) = self.module {
            println!("Linking module '{}'...", module_name);
            linker::link_module(
                &config,
                &mut state,
                module_name,
                profile.as_deref(),
                &hostname,
            )?;
        }

        state.last_sync = Some(chrono::Local::now().to_rfc3339());
        state.save()?;
        println!("State saved.");

        Ok(())
    }
}

fn resolve_profile<'a>(
    config: &'a Config,
    state: &'a State,
    hostname: &str,
    override_profile: Option<&'a str>,
) -> Result<Option<String>> {
    // Priority: CLI flag > state > host default
    if let Some(p) = override_profile {
        if !config.profiles.contains_key(p) {
            return Err(eyre!("profile '{}' not found in config", p));
        }
        return Ok(Some(p.to_string()));
    }

    if let Some(ref p) = state.active_profile
        && config.profiles.contains_key(p.as_str())
    {
        return Ok(Some(p.clone()));
    }

    if let Some(host) = config.hosts.get(hostname)
        && let Some(ref dp) = host.default_profile
    {
        return Ok(Some(dp.clone()));
    }

    Ok(None)
}
