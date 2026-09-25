use color_eyre::eyre::Result;

use crate::{linker, state::State, ui};

#[derive(clap::Parser, Debug)]
pub struct UnlinkCommand {
    /// Module name to unlink
    module: String,
}

impl UnlinkCommand {
    pub fn run(&self) -> Result<()> {
        let mut state = State::load()?;

        ui::step(format!("Unlinking module '{}'...", self.module));
        linker::unlink_module(&mut state, &self.module)?;

        state.last_sync = Some(chrono::Local::now().to_rfc3339());
        state.save()?;
        ui::success("State saved.");

        Ok(())
    }
}
