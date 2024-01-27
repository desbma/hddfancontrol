//! Control fan speed according to drive temperature

use anyhow::Context;
use clap::Parser;

mod cl;
mod fan;
mod pwm;

use crate::fan::{Fan, Speed as FanSpeed};

fn main() -> anyhow::Result<()> {
    // Parse cl args
    let args = cl::Args::parse();

    // Init logger
    simple_logger::init_with_level(args.verbosity).context("Failed to init logger")?;

    match args.command {
        cl::Command::PwmTest { pwm } => {
            for pwm_path in &pwm {
                let mut fan = Fan::new(pwm_path)?;
                fan.set_speed(FanSpeed(255))?;
                todo!();
            }
        }
        cl::Command::Daemon { .. } => {
            todo!();
        }
    }

    Ok(())
}
