//! Command line interface

use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

use crate::pwm;

/// Device temperature
pub type Temperature = u8;
/// Speed percentage
pub type Percentage = u8;

/// PWM operation settings
#[derive(Clone, Debug)]
pub struct PwmSettings {
    /// Sysfs filepath
    pub filepath: PathBuf,
    /// Minimum value at which the fans start moving
    pub start: pwm::Value,
    /// Maximum value at which the fans stop moving
    pub stop: pwm::Value,
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
            start,
            stop,
        })
    }
}

/// Parse percentage integer value
fn percentage(s: &str) -> Result<u8, String> {
    clap_num::number_range(s, 0, 100)
}

/// Hddfancontrol command line arguments
#[derive(Parser, Debug)]
pub struct Args {
    /// Level of logging output (TRACE, DEBUG, INFO, WARN, ERROR).
    #[arg(short, default_value_t = log::Level::Info)]
    pub verbosity: log::Level,

    /// Main action
    #[command(subcommand)]
    pub command: Command,
}

/// Main command
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start fan control daemon
    Daemon {
        /// Drive(s) to get temperature from (ie. /dev/sdX).
        #[arg(short, long, num_args = 1.., required = true)]
        drives: Vec<PathBuf>,

        /// PWM filepath(s) with values at which the fan start and stop moving.
        /// Use the 'pwm-test' command to find these values.
        /// (ie. /sys/class/hwmon/hwmonX/device/pwmY:200:75).
        #[arg(short, long, num_args = 1.., required = true)]
        pwm: Vec<PwmSettings>,

        /// Temperatures in Celcius at which the fan(s) will be set to minimum/maximum speed.
        #[arg(short, long, num_args = 2, default_values_t = vec![30, 50])]
        temp_range: Vec<Temperature>,

        /// Minimum percentage of full fan speed to set the fan to.
        /// Never set to 0 unless you have other fans to cool down your system,
        /// or a case specially designed for passive cooling..
        #[arg(short, long, default_value_t = 20, value_parser=percentage)]
        min_fan_speed_prct: Percentage,

        /// Interval to check temperature and adjust fan speed, ie. '30s', '3min'.
        #[arg(short, long)]
        interval: humantime::Duration,

        /// Also control fan speed according to this CPU temperature probe.
        /// (ie. `/sys/devices/platform/coretemp.0/hwmon/hwmonX/tempY_input`).
        /// WARNING: Only use for low TDP CPUs. You may need to set
        /// a low value for -i/--interval parameter to react quickly to sudden CPU temperature increase.
        #[arg(short, long)]
        cpu_sensor: Option<PathBuf>,

        /// CPU temperature range, if CPU temp monitoring is enabled.
        /// If missing, will be autodetected or use a default value.
        #[arg(long, num_args = 2)]
        cpu_temp_range: Option<Vec<Temperature>>,

        /// hddtemp daemon TCP port.
        #[arg(long, default_value_t = 7634)]
        hddtemp_daemon_port: u16,

        /// Restore fan settings on exit, otherwise the fans are run at full speed on exit.
        #[arg(short, long)]
        restore_fan_settings: bool,
    },

    /// Test PWM to find start/stop fan values
    PwmTest {
        /// PWM filepath(s) (ie. /sys/class/hwmon/hwmonX/device/pwmY).
        #[arg(short, long, num_args = 1.., required = true)]
        pwm: Vec<PathBuf>,
    },
}
