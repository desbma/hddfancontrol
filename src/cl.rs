//! Command line interface

use std::{
    fmt, fs, io,
    ops::Range,
    path::{self, PathBuf},
    str::FromStr,
};

use clap::{Parser, Subcommand};

use crate::{fan::Thresholds, probe::Temp};

/// Speed percentage
pub(crate) type Percentage = u8;

/// PWM operation settings
#[derive(Clone, Debug)]
pub(crate) struct PwmSettings {
    /// Sysfs filepath
    pub filepath: PathBuf,
    /// Fan characteristics
    pub thresholds: Thresholds,
}

impl FromStr for PwmSettings {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.rsplitn(3, ':');
        let stop = tokens
            .next()
            .ok_or("Missing stop value")?
            .parse()
            .map_err(|_| "Invalid stop value")?;
        let start = tokens
            .next()
            .ok_or("Missing start value")?
            .parse()
            .map_err(|_| "Invalid start value")?;
        let filepath = tokens.next().ok_or("Missing filepath")?.into();
        Ok(Self {
            filepath,
            thresholds: Thresholds {
                min_start: start,
                max_stop: stop,
            },
        })
    }
}

/// Hwmon path and temperature range
#[derive(Clone, Debug)]
pub(crate) struct HwmonSettings {
    /// Sysfs filepath
    pub filepath: PathBuf,
    /// Temperature range
    pub temp: Option<Range<Temp>>,
}

impl FromStr for HwmonSettings {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.splitn(3, ':');
        let filepath = tokens.next().ok_or("Missing filepath")?.into();
        let start = tokens
            .next()
            .map(str::parse)
            .transpose()
            .map_err(|_| "Invalid min speed temp value")?;
        let end = tokens
            .next()
            .map(str::parse)
            .transpose()
            .map_err(|_| "Invalid max speed temp value")?;
        Ok(Self {
            filepath,
            temp: if let (Some(start), Some(end)) = (start, end) {
                Some(Range { start, end })
            } else {
                None
            },
        })
    }
}

/// Parse percentage integer value
fn percentage(s: &str) -> Result<u8, String> {
    clap_num::number_range(s, 0, 100)
}

/// Hddfancontrol command line arguments
#[derive(Parser, Debug)]
#[command(version, about)]
pub(crate) struct Args {
    /// Level of logging output (TRACE, DEBUG, INFO, WARN, ERROR).
    #[arg(short, default_value_t = log::Level::Info)]
    pub verbosity: log::Level,

    /// Main action
    #[command(subcommand)]
    pub command: Command,
}

/// Drive selector matching 0 or more drives
#[derive(Clone, Debug)]
pub(crate) enum DriveSelector {
    /// All drives for an interface type
    Interface(String),
    /// A single drive path
    DrivePath(PathBuf),
}

impl DriveSelector {
    /// Resolve selector to drive paths
    pub(crate) fn to_drive_paths(&self) -> io::Result<Vec<PathBuf>> {
        match self {
            DriveSelector::Interface(itf) => {
                let drives = fs::read_dir("/dev/disk/by-id")?.collect::<io::Result<Vec<_>>>()?;
                let prefix = format!("{itf}-");
                Ok(drives
                    .into_iter()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name().and_then(|f| f.to_str()).is_some_and(|f| {
                            f.starts_with(&prefix)
                                && !f.trim_end_matches(char::is_numeric).ends_with("-part")
                        })
                    })
                    .collect())
            }
            DriveSelector::DrivePath(p) => Ok(vec![p.to_owned()]),
        }
    }
}

impl fmt::Display for DriveSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriveSelector::Interface(itf) => write!(f, "{itf}"),
            DriveSelector::DrivePath(p) => write!(f, "{p:?}"),
        }
    }
}

impl FromStr for DriveSelector {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(path::MAIN_SEPARATOR) {
            Ok(Self::DrivePath(s.into()))
        } else {
            Ok(Self::Interface(s.to_owned()))
        }
    }
}

/// Main command
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Start fan control daemon
    Daemon {
        /// Drive path(s) to get temperature from (ie. `/dev/sdX`).
        /// Interface type selectors are also supported (ie. `ata` to
        /// select all drives matching `/dev/disk/by-id/ata-*`).
        #[arg(short, long, num_args = 1.., required = true)]
        drives: Vec<DriveSelector>,

        /// PWM filepath(s) with values at which the fan start and stop moving.
        /// Use the 'pwm-test' command to find these values.
        /// Format is `PWM_PATH:STAT_VAL:STOP_VAL`
        /// (ie. `/sys/class/hwmon/hwmonX/device/pwmY:200:75`)
        #[arg(short, long, num_args = 1.., required = true)]
        pwm: Vec<PwmSettings>,

        /// Temperatures in Celcius at which the fan(s) will be set to minimum/maximum speed.
        #[arg(short = 't', long, value_name = "TEMP", num_args = 2, default_values_t = vec![30.0, 50.0])]
        drive_temp_range: Vec<Temp>,

        /// Minimum percentage of full fan speed to set the fan to.
        /// Never set to 0 unless you have other fans to cool down your system,
        /// or a case specially designed for passive cooling.
        #[arg(short, long, default_value_t = 20, value_parser=percentage)]
        min_fan_speed_prct: Percentage,

        /// Interval to check temperature and adjust fan speed, ie. '30s', '3min'.
        #[arg(short, long, default_value = "20s")]
        interval: humantime::Duration,

        /// Also control fan speed according to these additional hwmon temperature probes.
        /// Format is `HWMON_PATH[:TEMP_MIN_SPEED:TEMP_MAX_SPEED]`
        /// (ie. `/sys/devices/platform/coretemp.0/hwmon/hwmonX/tempY_input:45:75`).
        /// If missing, target temperature range will be autodetected or use a default value.
        /// WARNING: Don't use for CPU sensors, unless you have low TDP CPU. You may also need to set
        /// a low value for -i/--interval parameter to react quickly to sudden temperature increase.
        #[arg(short = 'w', long)]
        hwmons: Vec<HwmonSettings>,

        /// hddtemp daemon TCP port.
        #[arg(long, default_value_t = 7634)]
        hddtemp_daemon_port: u16,

        /// Restore fan settings on exit, otherwise the fans are run at full speed on exit.
        #[arg(short, long)]
        restore_fan_settings: bool,
    },

    /// Test PWM to find start/stop fan values
    PwmTest {
        /// PWM filepath(s) (ie. `/sys/class/hwmon/hwmonX/device/pwmY`).
        #[arg(short, long, num_args = 1.., required = true)]
        pwm: Vec<PathBuf>,
    },
}
