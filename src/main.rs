//! Control fan speed according to drive temperature

use anyhow::Context;
use clap::Parser;

mod cl;
mod device;
mod fan;
mod probe;
mod pwm;
#[cfg(test)]
mod tests;

use crate::{device::Drive, fan::Fan};

fn main() -> anyhow::Result<()> {
    // Parse cl args
    let args = cl::Args::parse();

    // Init logger
    simple_logger::init_with_level(args.verbosity).context("Failed to init logger")?;

    match args.command {
        cl::Command::PwmTest { pwm } => {
            for pwm_path in &pwm {
                let mut fan = Fan::new(pwm_path)?;
                log::info!("Testing fan {fan}, this may take a long time");
                match fan.test() {
                    Ok(t) => {
                        log::info!("Fan {fan}] start/stop thresholds: {t}");
                    }
                    Err(e) => {
                        log::error!("Fan {fan} test failed: {e}");
                    }
                }
            }
        }
        cl::Command::Daemon { drives, .. } => {
            for drive_path in drives {
                let drive = Drive::new(&drive_path)?;
                let mut prober = probe::prober(&drive)?.ok_or_else(|| {
                    anyhow::anyhow!("No probing method found for drive {drive_path:?}")
                })?;
                let _temp = prober.probe_temp()?;
                // dbg!(temp);
            }
            todo!();
        }
    }

    Ok(())
}
