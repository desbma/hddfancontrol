//! Control fan speed according to drive temperature

#![cfg_attr(
    feature = "gen-man-pages",
    expect(dead_code, unused_crate_dependencies, unused_imports)
)]

use std::{
    collections::VecDeque,
    ops::Range,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use anyhow::Context as _;
use clap::Parser as _;
use device::Hwmon;
use exit::ExitHook;
use fan::{Fan, Speed};
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

use crate::{
    device::Drive,
    fan::{CommandFan, PwmFan},
    probe::DeviceTempProber,
};

/// Interruptible sleep
fn sleep(dur: Duration, exit_rx: &mpsc::Receiver<()>) {
    let _ = exit_rx.recv_timeout(dur);
}

#[cfg(feature = "gen-man-pages")]
fn main() -> anyhow::Result<()> {
    use clap::CommandFactory as _;
    let cmd = cl::Args::command();
    let output = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Missing output dir argument"))?;
    clap_mangen::generate_to(cmd, output)?;
    Ok(())
}

#[cfg(not(feature = "gen-man-pages"))]
#[expect(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    // Parse cl args
    let args = cl::Args::parse();

    // Init logger
    simple_logger::init_with_level(args.verbosity).context("Failed to init logger")?;

    match args.command {
        cl::Command::PwmTest { pwm } => {
            for pwm_path in &pwm {
                let fan = PwmFan::new(&cl::PwmSettings {
                    filepath: pwm_path.to_owned(),
                    thresholds: fan::Thresholds::default(),
                })
                .context("Failed to setup fan")?;
                let rpm_path = fan
                    .resolve_rpm_path()
                    .context("Failed to resolve fan rpm filepath")?;
                let mut fan = fan
                    .with_rpm_file(&rpm_path)
                    .context("Failed to setup fan with rpm filepath")?;
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
            drives: drive_selectors,
            hddtemp_daemon_port,
            pwm,
            fan_cmd,
            drive_temp_range,
            min_fan_speed_prct,
            interval,
            average,
            hwmons,
            restore_fan_settings,
        } => {
            #[expect(clippy::indexing_slicing)] // guaranteed by clap's numl_args
            let drive_temp_range = Range {
                start: drive_temp_range[0],
                end: drive_temp_range[1],
            };
            let drive_paths: Vec<PathBuf> = drive_selectors
                .into_iter()
                .map(|s| {
                    s.to_drive_paths()
                        .with_context(|| format!("Failed to match drives for selector {s}"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect();
            anyhow::ensure!(!drive_paths.is_empty(), "No drive match");
            let drives: Vec<Drive> = drive_paths
                .iter()
                .map(|path| Drive::new(path))
                .collect::<anyhow::Result<_>>()
                .context("Failed to setup drives")?;
            let mut drive_probers: Vec<(Box<dyn DeviceTempProber>, bool)> = drives
                .iter()
                .zip(drive_paths.iter())
                .map(|(drive, path)| {
                    probe::prober(drive, hddtemp_daemon_port)
                        .with_context(|| format!("Failed to setup prober for drive {drive}"))?
                        .ok_or_else(|| {
                            anyhow::anyhow!("No probing method found for drive {path:?}")
                        })
                })
                .collect::<anyhow::Result<_>>()
                .context("Failed to setup drive probers")?;

            let mut hwmon_and_range: Vec<(Hwmon, Range<Temp>)> = hwmons
                .iter()
                .map(|h| {
                    let hwm = Hwmon::new(&h.filepath)
                        .with_context(|| format!("Failed to setup hwmon {:?}", h.filepath))?;
                    let range = h.temp.as_ref().map_or_else(
                        || -> anyhow::Result<_> {
                            // Default range
                            let range = hwm.default_range().with_context(|| {
                                format!(
                                    "Failed to compute default temperature range for hwmon {hwm}"
                                )
                            })?;
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

            let min_fan_speed = Speed::try_from(f64::from(min_fan_speed_prct) / 100.0)
                .with_context(|| format!("Invalid speed {min_fan_speed_prct}%"))?;
            let mut fans: Vec<Box<dyn Fan>> = pwm
                .iter()
                .map(|p| PwmFan::new(p).map(|f| Box::new(f) as Box<dyn Fan>))
                .collect::<anyhow::Result<_>>()
                .context("Failed to setup PWM fans")?;
            let cmd_fans: Vec<Box<dyn Fan>> = fan_cmd
                .iter()
                .map(|c| Ok(Box::new(CommandFan::new(c)) as Box<dyn Fan>))
                .collect::<anyhow::Result<_>>()
                .context("Failed to setup command fans")?;
            fans.extend(cmd_fans);

            let _exit_hook = ExitHook::new(
                pwm.iter()
                    .map(|p| pwm::Pwm::new(&p.filepath))
                    .collect::<anyhow::Result<_>>()
                    .context("Failed to setup PWMs for exit hook")?,
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
                })
                .context("Failed to setup SIGINT handler")?;
            }

            let sample_count = average.get();
            let mut temp_measures: VecDeque<Temp> = VecDeque::with_capacity(sample_count);

            while !exit_requested.load(Ordering::SeqCst) {
                let start = Instant::now();

                let max_drive_temp = drive_probers
                    .iter_mut()
                    .zip(drives.iter())
                    .map(|((prober, supports_probing_sleeping), drive)| {
                        let state = drive
                            .state()
                            .with_context(|| format!("Failed to get drive {drive} state"))?;
                        log::debug!("Drive {drive} state: {state}");
                        let temp = if state.can_probe_temp(*supports_probing_sleeping) {
                            let temp = prober
                                .probe_temp()
                                .with_context(|| format!("Failed to get drive {drive} temp"))?;
                            log::debug!("Drive {drive}: {temp}°C");
                            Some(temp)
                        } else {
                            log::debug!("Drive {drive} in state {state} can not be probed");
                            None
                        };
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()
                    .context("Failed to get maximum drive temperature")?
                    .into_iter()
                    .flatten()
                    .reduce(f64::max);

                let hwmon_temps: Vec<Temp> = hwmon_and_range
                    .iter_mut()
                    .map(|(hwm, _range)| {
                        let temp = hwm
                            .probe_temp()
                            .with_context(|| format!("Failed to get hwmon {hwm} temp"))?;
                        log::info!("Hwmon {hwm} temperature: {temp}°C");
                        Ok(temp)
                    })
                    .collect::<anyhow::Result<_>>()?;

                let max_temp = max_drive_temp
                    .into_iter()
                    .chain(hwmon_temps)
                    .reduce(f64::max);

                if let Some(temp) = max_temp {
                    if temp_measures.len() == sample_count {
                        temp_measures.pop_front();
                    }
                    temp_measures.push_back(temp);
                }

                let speed = if temp_measures.is_empty() {
                    log::info!("All drives are spun down");
                    min_fan_speed
                } else {
                    #[expect(clippy::cast_precision_loss)]
                    let avg_temp = temp_measures.iter().sum::<Temp>() / temp_measures.len() as Temp;
                    log::info!("Avg max temperature: {avg_temp:.1}°C");
                    fan::target_speed(avg_temp, &drive_temp_range, min_fan_speed)
                };
                for fan in &mut fans {
                    fan.set_speed(speed)
                        .with_context(|| format!("Failed to set fan {fan} speed"))?;
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
