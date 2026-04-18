use color_eyre::eyre::Result;

use crate::{linker, state::State};

#[derive(clap::Parser, Debug)]
pub struct UnlinkCommand {
    /// Module name to unlink
    module: String,
}

impl UnlinkCommand {
    pub fn run(&self) -> Result<()> {
        let mut state = State::load()?;

        println!("Unlinking module '{}'...", self.module);
        linker::unlink_module(&mut state, &self.module)?;

        state.last_sync = Some(chrono::Local::now().to_rfc3339());
        state.save()?;
        println!("State saved.");

        Ok(())
    }
}
