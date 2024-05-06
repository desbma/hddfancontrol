//! Control fan speed according to drive temperature

use std::{cmp::max, ops::Range, thread::sleep, time::Instant};

use anyhow::Context;
use clap::Parser;
use fan::Speed;
use probe::Temp;

mod cl;
mod device;
mod fan;
mod probe;
mod pwm;
#[cfg(test)]
mod tests;

use crate::{device::Drive, fan::Fan, probe::DriveTempProber};

/// Compute target fan speed for the given temp and parameters
fn target_fan_speed(temp: Temp, temp_range: &Range<Temp>, min_speed: Speed) -> Speed {
    if temp_range.contains(&temp) {
        let s = Speed::from_max_division_f64(
            temp - temp_range.start,
            temp_range.end - temp_range.start,
        );
        max(min_speed, s)
    } else if temp < temp_range.start {
        min_speed
    } else {
        Speed::MAX
    }
}

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
        cl::Command::Daemon {
            drives: drive_paths,
            hddtemp_daemon_port,
            pwm,
            temp_range,
            min_fan_speed_prct,
            interval,
            // cpu_sensor: _,
            // cpu_temp_range: _,
            // spin_down_time: _,
            // restore_fan_settings: _,
            ..
        } => {
            let temp_range = Range {
                start: f64::from(temp_range[0]),
                end: f64::from(temp_range[1]),
            };
            let min_fan_speed = Speed::from_max_division_u8(min_fan_speed_prct, 100);

            let drives: Vec<Drive> = drive_paths
                .iter()
                .map(|path| Drive::new(path))
                .collect::<anyhow::Result<_>>()?;
            let mut drive_probers: Vec<Box<dyn DriveTempProber>> = drives
                .iter()
                .zip(drive_paths.iter())
                .map(|(drive, path)| {
                    probe::prober(drive, hddtemp_daemon_port)?.ok_or_else(|| {
                        anyhow::anyhow!("No probing method found for drive {path:?}")
                    })
                })
                .collect::<anyhow::Result<_>>()?;

            let mut fans: Vec<_> = pwm
                .iter()
                .map(|p| Fan::new(&p.filepath))
                .collect::<anyhow::Result<_>>()?;

            loop {
                let start = Instant::now();

                #[allow(clippy::unwrap_used)]
                let max_temp = drive_probers
                    .iter_mut()
                    .zip(drives.iter())
                    .map(|(prober, drive)| {
                        let temp = prober.probe_temp()?;
                        log::debug!("Drive {}: {}°C", drive, temp);
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
                    .into_iter()
                    .reduce(f64::max)
                    .unwrap();
                log::info!("Max temp: {max_temp}°C");

                let speed = target_fan_speed(max_temp, &temp_range, min_fan_speed);
                for fan in &mut fans {
                    fan.set_speed(speed)?;
                }

                let elapsed = Instant::now().duration_since(start);
                let to_wait = interval.saturating_sub(elapsed);
                log::debug!("Will sleep at most {to_wait:?}");
                sleep(to_wait);
            }
        }
    }

    Ok(())
}
