//! Control fan speed according to drive temperature

use std::{ops::Range, thread::sleep, time::Instant};

use anyhow::Context;
use clap::Parser;
use device::Cpu;
use exit::ExitHook;
use fan::Speed;

mod cl;
mod device;
mod exit;
mod fan;
mod probe;
mod pwm;
#[cfg(test)]
mod tests;

use crate::{device::Drive, fan::Fan, probe::DeviceTempProber};

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
            drive_temp_range,
            min_fan_speed_prct,
            interval,
            cpu_sensor,
            cpu_temp_range,
            restore_fan_settings,
            ..
        } => {
            let drive_temp_range = Range {
                start: f64::from(drive_temp_range[0]),
                end: f64::from(drive_temp_range[1]),
            };
            let drives: Vec<Drive> = drive_paths
                .iter()
                .map(|path| Drive::new(path))
                .collect::<anyhow::Result<_>>()?;
            let mut drive_probers: Vec<(Box<dyn DeviceTempProber>, bool)> = drives
                .iter()
                .zip(drive_paths.iter())
                .map(|(drive, path)| {
                    probe::prober(drive, hddtemp_daemon_port)?.ok_or_else(|| {
                        anyhow::anyhow!("No probing method found for drive {path:?}")
                    })
                })
                .collect::<anyhow::Result<_>>()?;

            let mut cpu_range = match (cpu_sensor.map(|s| Cpu::new(&s)), cpu_temp_range) {
                // Default range
                (Some(cpu), None) => {
                    let range = cpu.default_range()?;
                    log::info!(
                        "CPU temperature range set to {}-{}°C",
                        range.start,
                        range.end
                    );
                    Some((cpu, range))
                }
                // Range set by user
                (Some(cpu), Some(range)) => Some((
                    cpu,
                    Range {
                        start: f64::from(range[0]),
                        end: f64::from(range[1]),
                    },
                )),
                // No CPU
                (None, _) => None,
            };

            let min_fan_speed = Speed::from_max_division_u8(min_fan_speed_prct, 100);
            let mut fans: Vec<_> = pwm
                .iter()
                .map(|p| Fan::new(&p.filepath))
                .collect::<anyhow::Result<_>>()?;
            let _exit_hook = ExitHook::new(
                pwm.iter()
                    .map(|p| pwm::Pwm::new(&p.filepath))
                    .collect::<anyhow::Result<_>>()?,
                restore_fan_settings,
            );

            loop {
                let start = Instant::now();

                let max_drive_temp = drive_probers
                    .iter_mut()
                    .zip(drives.iter())
                    .map(|((prober, supports_probing_sleeping), drive)| {
                        let state = drive.state()?;
                        let temp = if state.is_spun_down() && !*supports_probing_sleeping {
                            log::debug!("Drive {} is sleeping", drive);
                            None
                        } else {
                            let temp = prober.probe_temp()?;
                            log::debug!("Drive {}: {}°C", drive, temp);
                            Some(temp)
                        };
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .reduce(f64::max);

                let cpu_temp = cpu_range
                    .as_mut()
                    .map(|(cpu, _range)| -> anyhow::Result<_> {
                        let temp = cpu.probe_temp()?;
                        log::info!("CPU temperature: {}°C", temp);
                        Ok(temp)
                    })
                    .map_or(Ok(None), |v| v.map(Some))?;

                let mut speed = min_fan_speed;
                if let Some(max_drive_temp) = max_drive_temp {
                    log::info!("Max drive temperature: {max_drive_temp}°C");
                    speed = fan::target_speed(max_drive_temp, &drive_temp_range, speed);
                } else {
                    log::info!("All drives are spun down");
                }
                if let (Some(cpu_temp), Some((_, temp_range))) = (cpu_temp, cpu_range.as_ref()) {
                    speed = fan::target_speed(cpu_temp, temp_range, speed);
                }
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
