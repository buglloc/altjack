use anyhow::Result;

pub type CommandResult = Result<()>;

pub mod attiny;
pub mod ina219;
pub mod list;
pub mod power;
pub mod state;
