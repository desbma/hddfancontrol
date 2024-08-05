//! Control fan speed according to drive temperature

use std::{
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use clap::Parser;
use device::Hwmon;
use exit::ExitHook;
use fan::Speed;
use probe::Temp;

mod cl;
mod device;
mod exit;
mod fan;
mod probe;
mod pwm;
mod sysfs;
#[cfg(test)]
mod tests;

use crate::{device::Drive, fan::Fan, probe::DeviceTempProber};

/// Interruptible sleep
fn sleep(dur: Duration, exit_rx: &mpsc::Receiver<()>) {
    let _ = exit_rx.recv_timeout(dur);
}

#[expect(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    // Parse cl args
    let args = cl::Args::parse();

    // Init logger
    simple_logger::init_with_level(args.verbosity).context("Failed to init logger")?;

    match args.command {
        cl::Command::PwmTest { pwm } => {
            for pwm_path in &pwm {
                let mut fan = Fan::new(&cl::PwmSettings {
                    filepath: pwm_path.to_owned(),
                    // Unused
                    thresholds: fan::Thresholds {
                        min_start: 0,
                        max_stop: 0,
                    },
                })?;
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
            hwmons,
            restore_fan_settings,
        } => {
            let drive_temp_range = Range {
                start: drive_temp_range[0],
                end: drive_temp_range[1],
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

            let mut hwmon_and_range: Vec<(Hwmon, Range<Temp>)> = hwmons
                .iter()
                .map(|h| {
                    let hwm = Hwmon::new(&h.filepath)?;
                    let range = h.temp.as_ref().map_or_else(
                        || -> anyhow::Result<_> {
                            // Default range
                            let range = hwm.default_range()?;
                            log::info!(
                                "Device temperature range set to {}-{}°C",
                                range.start,
                                range.end
                            );
                            Ok(range)
                        },
                        |r| Ok(r.clone()),
                    )?;
                    Ok((hwm, range))
                })
                .collect::<anyhow::Result<_>>()?;

            let min_fan_speed = Speed::from_max_division(f64::from(min_fan_speed_prct), 100.0);
            let mut fans: Vec<_> = pwm.iter().map(Fan::new).collect::<anyhow::Result<_>>()?;

            let _exit_hook = ExitHook::new(
                pwm.iter()
                    .map(|p| pwm::Pwm::new(&p.filepath))
                    .collect::<anyhow::Result<_>>()?,
                restore_fan_settings,
            )?;

            // Signal handling
            let exit_requested = Arc::new(AtomicBool::new(false));
            let (exit_tx, exit_rx) = mpsc::channel::<()>();
            {
                let exit_requested = Arc::clone(&exit_requested);
                ctrlc::set_handler(move || {
                    exit_requested.store(true, Ordering::SeqCst);
                    let _ = exit_tx.send(());
                })?;
            }

            while !exit_requested.load(Ordering::SeqCst) {
                let start = Instant::now();

                let max_drive_temp = drive_probers
                    .iter_mut()
                    .zip(drives.iter())
                    .map(|((prober, supports_probing_sleeping), drive)| {
                        let state = drive.state()?;
                        log::debug!("Drive {drive} state: {state}");
                        let temp = if state.is_spun_down() && !*supports_probing_sleeping {
                            log::debug!("Drive {drive} is sleeping");
                            None
                        } else {
                            let temp = prober.probe_temp()?;
                            log::debug!("Drive {drive}: {temp}°C");
                            Some(temp)
                        };
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .reduce(f64::max);

                let hwmon_temps: Vec<Temp> = hwmon_and_range
                    .iter_mut()
                    .map(|(hwm, _range)| {
                        let temp = hwm.probe_temp()?;
                        log::info!("Hwmon {hwm} temperature: {temp}°C");
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<_>>()?;

                let mut speed = min_fan_speed;
                if let Some(max_drive_temp) = max_drive_temp {
                    log::info!("Max drive temperature: {max_drive_temp}°C");
                    speed = fan::target_speed(max_drive_temp, &drive_temp_range, speed);
                } else {
                    log::info!("All drives are spun down");
                }
                for (hwmon_temp, (_hwmon, hwmon_range)) in
                    hwmon_temps.into_iter().zip(hwmon_and_range.iter())
                {
                    speed = fan::target_speed(hwmon_temp, hwmon_range, speed);
                }
                for fan in &mut fans {
                    fan.set_speed(speed)?;
                }

                let elapsed = Instant::now().duration_since(start);
                let to_wait = interval.saturating_sub(elapsed);
                log::debug!("Will sleep at most {to_wait:?}");
                sleep(to_wait, &exit_rx);
            }
        }
    }

    Ok(())
}
