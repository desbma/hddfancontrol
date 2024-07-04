//! Fan control

#![allow(dead_code)]

use std::{
    cmp::{max, Ordering},
    fmt,
    ops::Range,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    cl::PwmSettings,
    probe::Temp,
    pwm::{self, ControlMode, Pwm},
};

/// Minimum duration to apply fan startup boost
const STARTUP_DELAY: Duration = Duration::from_secs(20);

/// Fan characteristics
#[derive(Clone, Debug)]
pub struct Thresholds {
    /// Minimum value at which the fan starts moving when it was stopped
    pub min_start: pwm::Value,
    /// Maximum value at which the fan stops moving when it was started
    pub max_stop: pwm::Value,
}

impl fmt::Display for Thresholds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.min_start, self.max_stop)
    }
}

/// Stateful fan
pub struct Fan {
    /// Fan pwm
    pwm: Pwm,
    /// Pwm thresholds
    thresholds: Thresholds,
    /// Current speed
    speed: Option<Speed>,
    /// Startup ts
    startup: Option<Instant>,
}

impl fmt::Display for Fan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.pwm.fmt(f)
    }
}

/// Fan speed as [0-1000] value
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Speed(u16);

impl Speed {
    /// Maximum speed value
    pub const MAX: Self = Self(1000);

    /// Minimum speed value
    pub const MIN: Self = Self(0);

    /// Build a speed with the value max * dividend / divisor
    pub fn from_max_division(dividend: f64, divisor: f64) -> Self {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Self((f64::from(Self::MAX.0) * dividend / divisor) as u16)
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:.1}%", f64::from(self.0) / 10.0)
    }
}

/// Speed change direction
#[derive(Copy, Clone)]
enum SpeedChange {
    /// Speed is increasing
    Increasing,
    /// Speed is decreasing
    Decreasing,
}

impl Fan {
    /// Build a new fan from PWM settings
    pub fn new(pwm_info: &PwmSettings) -> anyhow::Result<Self> {
        let pwm = Pwm::new(&pwm_info.filepath)?;
        Ok(Self {
            pwm,
            thresholds: pwm_info.thresholds.clone(),
            speed: None,
            startup: None,
        })
    }

    /// Set fan speed
    pub fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        if self.speed.map_or(true, |c| c != speed) {
            let prev_mode = self.pwm.get_mode()?;
            let new_mode = ControlMode::Software;
            if prev_mode != new_mode {
                self.pwm.set_mode(new_mode)?;
                log::info!(
                    "PWM {} mode set from {} to {}",
                    self.pwm,
                    prev_mode,
                    new_mode
                );
            }
            let mut pwm_value = self.thresholds.max_stop
                + (u16::from(pwm::Value::MAX - self.thresholds.max_stop) * speed.0 / Speed::MAX.0)
                    as u8;
            if self.speed == Some(Speed::MIN) {
                log::info!("Fan {} startup", self.pwm);
                pwm_value = max(pwm_value, self.thresholds.min_start);
                self.startup = Some(Instant::now());
            } else if self
                .startup
                .is_some_and(|s| Instant::now().duration_since(s) < STARTUP_DELAY)
            {
                pwm_value = max(pwm_value, self.thresholds.min_start);
            }
            self.pwm.set(pwm_value)?;
            log::info!("Fan {} speed set to {}", self.pwm, speed);
            self.speed = Some(speed);
        } else {
            log::trace!("Fan {} speed unchanged: {}", self.pwm, speed);
        }
        Ok(())
    }

    /// Wait until fan speed stop increasing or decreasing
    fn wait_stable(&self, change: SpeedChange) -> anyhow::Result<()> {
        /// Maximum duration to wait for the fan to be stabilized
        const STABILIZE_TIMEOUT: Duration = Duration::from_secs(30);
        /// Probe interval
        const STABILIZE_PROBE_DELAY: Duration = Duration::from_millis(2000);

        let mut time_waited = Duration::from_secs(0);
        let mut prev_rpm = self.pwm.get_rpm()?;
        debug_assert!((prev_rpm > 0) || matches!(change, SpeedChange::Increasing));
        loop {
            sleep(STABILIZE_PROBE_DELAY);
            time_waited += STABILIZE_PROBE_DELAY;

            let cur_rpm = self.pwm.get_rpm()?;
            log::debug!("Fan {self} RPM: {cur_rpm}");

            // We consider the fan speed stable if it changed less than 10% (if the value is significant),
            // and if the direction changed
            if (cur_rpm < 100) || (cur_rpm.abs_diff(prev_rpm) < (cur_rpm / 10)) {
                #[allow(clippy::match_same_arms)]
                match (cur_rpm.cmp(&prev_rpm), change) {
                    (Ordering::Equal, _) => break,
                    (Ordering::Greater, SpeedChange::Decreasing) => break,
                    (Ordering::Less, SpeedChange::Increasing) => break,
                    _ => (),
                }
            }

            anyhow::ensure!(
                time_waited < STABILIZE_TIMEOUT,
                "Fan did not stabilize after {STABILIZE_TIMEOUT:?}"
            );

            prev_rpm = cur_rpm;
        }
        Ok(())
    }

    /// Is the fan physically moving?
    fn is_moving(&self) -> anyhow::Result<bool> {
        Ok(self.pwm.get_rpm()? > 0)
    }

    /// Dynamically test fan to find its thresholds
    pub fn test(&mut self) -> anyhow::Result<Thresholds> {
        self.set_speed(Speed::MAX)?;
        self.wait_stable(SpeedChange::Increasing)?;
        anyhow::ensure!(self.is_moving()?, "Fan is not moving at maximum speed");

        let mut max_stop = 0;
        for pwm_val in (0..=pwm::Value::MAX).rev().step_by(5) {
            self.set_speed(Speed(
                Speed::MAX.0 * u16::from(pwm_val) / u16::from(pwm::Value::MAX),
            ))?;
            self.wait_stable(SpeedChange::Decreasing)?;
            if !self.is_moving()? {
                max_stop = pwm_val;
                break;
            }
        }
        anyhow::ensure!(!self.is_moving()?, "Fan still moves at minimum speed");

        let mut min_start = 0;
        for pwm_val in (0..=u8::MAX).step_by(5) {
            self.set_speed(Speed(
                Speed::MAX.0 * u16::from(pwm_val) / u16::from(pwm::Value::MAX),
            ))?;
            self.wait_stable(SpeedChange::Increasing)?;
            if self.is_moving()? {
                min_start = pwm_val;
                break;
            }
        }
        anyhow::ensure!(self.is_moving()?, "Fan is not moving at maximum speed");

        Ok(Thresholds {
            min_start,
            max_stop,
        })
    }
}

/// Compute target fan speed for the given temp and parameters
pub fn target_speed(temp: Temp, temp_range: &Range<Temp>, min_speed: Speed) -> Speed {
    if temp_range.contains(&temp) {
        let s =
            Speed::from_max_division(temp - temp_range.start, temp_range.end - temp_range.start);
        max(min_speed, s)
    } else if temp < temp_range.start {
        min_speed
    } else {
        Speed::MAX
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {

    use super::*;

    #[test]
    fn test_target_speed() {
        assert_eq!(
            target_speed(
                45.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(20.0, 100.0)
            ),
            Speed(500)
        );
        assert_eq!(
            target_speed(
                40.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(20.0, 100.0)
            ),
            Speed(200)
        );
        assert_eq!(
            target_speed(
                35.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(20.0, 100.0)
            ),
            Speed(200)
        );
        assert_eq!(
            target_speed(
                40.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(0.0, 100.0)
            ),
            Speed::MIN
        );
        assert_eq!(
            target_speed(
                35.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(0.0, 100.0)
            ),
            Speed::MIN
        );
        assert_eq!(
            target_speed(
                50.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(20.0, 100.0)
            ),
            Speed::MAX
        );
        assert_eq!(
            target_speed(
                55.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::from_max_division(20.0, 100.0)
            ),
            Speed::MAX
        );
    }
}
