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
#[cfg(feature = "temp_log")]
mod temp_log;
#[cfg(test)]
mod tests;

use crate::{
    device::Drive,
    fan::{CommandFan, PwmFan},
    probe::DeviceTempProber,
};

/// A temperature prober paired with whether it supports probing sleeping drives
type DriveProber = (Box<dyn DeviceTempProber>, bool);

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

/// Run PWM fan test
fn run_pwm_test(pwm: &[PathBuf]) -> anyhow::Result<()> {
    for pwm_path in pwm {
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
    Ok(())
}

/// Set up drives and their temperature probers
fn setup_drives(
    drive_selectors: Vec<cl::DriveSelector>,
    hddtemp_daemon_port: u16,
) -> anyhow::Result<(Vec<Drive>, Vec<DriveProber>)> {
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
    let drive_probers: Vec<DriveProber> = drives
        .iter()
        .zip(drive_paths.iter())
        .map(|(drive, path)| {
            probe::prober(drive, hddtemp_daemon_port)
                .with_context(|| format!("Failed to setup prober for drive {drive}"))?
                .ok_or_else(|| anyhow::anyhow!("No probing method found for drive {path:?}"))
        })
        .collect::<anyhow::Result<_>>()
        .context("Failed to setup drive probers")?;
    Ok((drives, drive_probers))
}

/// Set up hwmon sensors with their temperature ranges
fn setup_hwmons(hwmons: &[cl::HwmonSettings]) -> anyhow::Result<Vec<(Hwmon, Range<Temp>)>> {
    hwmons
        .iter()
        .map(|h| {
            let hwm = Hwmon::new(&h.filepath)
                .with_context(|| format!("Failed to setup hwmon {:?}", h.filepath))?;
            let range = h.temp.as_ref().map_or_else(
                || -> anyhow::Result<_> {
                    // Default range
                    let range = hwm.default_range().with_context(|| {
                        format!("Failed to compute default temperature range for hwmon {hwm}")
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
        .collect::<anyhow::Result<_>>()
}

/// Set up all fans (PWM and command-based)
fn setup_fans(
    pwm: &[cl::PwmSettings],
    fan_cmd: &[std::ffi::OsString],
) -> anyhow::Result<Vec<Box<dyn Fan>>> {
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
    Ok(fans)
}

/// Probe temperature for each drive, returning `None` for sleeping drives
fn probe_drive_temps(
    drive_probers: &mut [DriveProber],
    drives: &[Drive],
) -> anyhow::Result<Vec<Option<Temp>>> {
    drive_probers
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
        .context("Failed to get drive temperatures")
}

/// Probe temperatures from hwmon sensors
fn probe_hwmon_temps(hwmon_and_range: &mut [(Hwmon, Range<Temp>)]) -> anyhow::Result<Vec<Temp>> {
    hwmon_and_range
        .iter_mut()
        .map(|(hwm, _range)| {
            let temp = hwm
                .probe_temp()
                .with_context(|| format!("Failed to get hwmon {hwm} temp"))?;
            log::info!("Hwmon {hwm} temperature: {temp}°C");
            Ok(temp)
        })
        .collect::<anyhow::Result<_>>()
}

/// Run the fan control daemon
fn run_daemon(args: cl::DaemonArgs) -> anyhow::Result<()> {
    let drive_temp_range = args.drive_temp_range();
    let interval = *args.interval;
    let min_fan_speed = Speed::try_from(f64::from(args.min_fan_speed_prct) / 100.0)
        .with_context(|| format!("Invalid speed {}%", args.min_fan_speed_prct))?;
    let (drives, mut drive_probers) = setup_drives(args.drives, args.hddtemp_daemon_port)?;
    let mut hwmon_and_range = setup_hwmons(&args.hwmons)?;
    let mut fans = setup_fans(&args.pwm, &args.fan_cmd)?;
    #[cfg(feature = "temp_log")]
    let mut temp_log_writer = args
        .temp_log
        .as_deref()
        .map(temp_log::TempLogWriter::new)
        .transpose()
        .context("Failed to open temp log file")?;

    let _exit_hook = ExitHook::new(
        args.pwm
            .iter()
            .map(|p| pwm::Pwm::new(&p.filepath))
            .collect::<anyhow::Result<_>>()
            .context("Failed to setup PWMs for exit hook")?,
        args.restore_fan_settings,
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

    let sample_count = args.average.get();
    let mut temp_measures: VecDeque<Temp> = VecDeque::with_capacity(sample_count);

    while !exit_requested.load(Ordering::SeqCst) {
        let start = Instant::now();

        // Measure
        let drive_temps = probe_drive_temps(&mut drive_probers, &drives)?;
        let hwmon_temps = probe_hwmon_temps(&mut hwmon_and_range)?;

        // Log
        #[cfg(feature = "temp_log")]
        if let Some(writer) = temp_log_writer.as_mut() {
            let measures = drives
                .iter()
                .zip(drive_temps.iter())
                .map(|(drive, temp)| temp_log::TempMeasure::new(drive, *temp))
                .chain(
                    hwmon_and_range
                        .iter()
                        .map(|(h, _r)| h)
                        .zip(hwmon_temps.iter())
                        .map(|(hwmon, temp)| temp_log::TempMeasure::new(hwmon, Some(*temp))),
                )
                .collect();
            writer
                .write(measures)
                .context("Failed to write temp log entry")?;
        }

        // Keep max
        let max_temp = drive_temps
            .into_iter()
            .flatten()
            .chain(hwmon_temps)
            .reduce(f64::max);
        if let Some(temp) = max_temp {
            if temp_measures.len() == sample_count {
                temp_measures.pop_front();
            }
            temp_measures.push_back(temp);
        }

        // Compute fan speed
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

        // Sleep
        let elapsed = Instant::now().duration_since(start);
        let to_wait = interval.saturating_sub(elapsed);
        log::debug!("Will sleep at most {to_wait:?}");
        sleep(to_wait, &exit_rx);
    }

    Ok(())
}

#[cfg(not(feature = "gen-man-pages"))]
fn main() -> anyhow::Result<()> {
    // Parse cl args
    let args = cl::Args::parse();

    // Init logger
    simple_logger::init_with_level(args.verbosity).context("Failed to init logger")?;

    match args.command {
        cl::Command::PwmTest { pwm } => run_pwm_test(&pwm),
        cl::Command::Daemon(daemon_args) => run_daemon(daemon_args),
    }
}
