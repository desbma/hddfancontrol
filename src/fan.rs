//! Fan control

use std::{
    cmp::{Ordering, max},
    fmt,
    ops::Range,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::Context as _;

use crate::{
    cl::PwmSettings,
    probe::Temp,
    pwm::{self, ControlMode, Pwm},
};

/// Minimum duration to apply fan startup boost
const STARTUP_DELAY: Duration = Duration::from_secs(20);

/// Fan characteristics
#[derive(Clone, Debug, Default)]
pub(crate) struct Thresholds {
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
pub(crate) struct Fan<T> {
    /// Fan pwm
    pwm: Pwm<T>,
    /// Pwm thresholds
    thresholds: Thresholds,
    /// Current speed
    speed: Option<Speed>,
    /// Startup ts
    startup: Option<Instant>,
}

impl<T> fmt::Display for Fan<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.pwm.fmt(f)
    }
}

/// Fan speed as [0-1] value
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Speed(typed_floats::PositiveFinite<f64>);

impl Speed {
    /// Test if speed is null
    pub(crate) fn is_zero(self) -> bool {
        self.0.is_positive_zero()
    }

    /// Get the speed value as f64
    pub(crate) fn get(self) -> f64 {
        self.0.get()
    }
}

#[expect(clippy::missing_docs_in_private_items)]
#[derive(thiserror::Error, Debug)]
pub(crate) enum SpeedConversionError {
    #[error("Value not in range [0.0; 1.0]")]
    Range,
    #[error("Invalid value: {0}")]
    InvalidNumber(typed_floats::InvalidNumber),
}

impl TryFrom<f64> for Speed {
    type Error = SpeedConversionError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if (0.0..=1.0).contains(&value) {
            Ok(Speed(
                typed_floats::PositiveFinite::<f64>::new(value)
                    .map_err(SpeedConversionError::InvalidNumber)?,
            ))
        } else {
            Err(SpeedConversionError::Range)
        }
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:.1}%", self.0.get() * 100.0)
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

impl Fan<()> {
    /// Build a new fan from PWM settings
    pub(crate) fn new(pwm_info: &PwmSettings) -> anyhow::Result<Self> {
        let pwm = Pwm::new(&pwm_info.filepath)?;
        Ok(Self {
            pwm,
            thresholds: pwm_info.thresholds.clone(),
            speed: None,
            startup: None,
        })
    }

    /// Find RPM filepath for the current fan
    pub(crate) fn resolve_rpm_path(&self) -> anyhow::Result<PathBuf> {
        /// Delay to wait for between PWM speed control, and RPM feedback to ensure both are correlated
        const RPM_CORRELATION_DELAY: Duration = Duration::from_secs(3);

        let dir = self.pwm.sysfs_dir();
        let candidates: Vec<_> = dir
            .read_dir()
            .with_context(|| format!("Failed to read directory {dir:?}"))?
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|f| f.starts_with("fan") && f.ends_with("_input"))
            })
            .map(|e| e.path())
            .collect();
        log::debug!("RPM file candidates for {}: {:?}", self.pwm, candidates);

        match candidates.len() {
            0 => Err(anyhow::anyhow!("Unable to find any fan RPM sysfs path")),
            1 =>
            {
                #[expect(clippy::unwrap_used)]
                Ok(candidates.into_iter().next().unwrap())
            }
            c => {
                log::info!(
                    "Running tests for {} candidates to find RPM file correlated with PWM {}, this may take some time",
                    c,
                    self.pwm
                );

                for candidate in candidates {
                    let pwm = self.pwm.clone().with_rpm_file(&candidate)?;

                    let mut skip = false;
                    for _ in 0..3 {
                        pwm.set(pwm::Value::MIN)?;
                        sleep(RPM_CORRELATION_DELAY);
                        if pwm.get_rpm()? > 0 {
                            log::debug!(
                                "RPM file {candidate:?} has positive value with PWM at minimum value, excluding"
                            );
                            skip = true;
                            break;
                        }

                        pwm.set(pwm::Value::MAX)?;
                        sleep(RPM_CORRELATION_DELAY);
                        if pwm.get_rpm()? == 0 {
                            log::debug!(
                                "RPM file {candidate:?} has null value with PWM at maximum value, excluding"
                            );
                            skip = true;
                            break;
                        }
                    }

                    if skip {
                        continue;
                    }

                    log::info!("RPM file for {} resolved to {:?}", self.pwm, candidate);
                    return Ok(candidate);
                }
                Err(anyhow::anyhow!("Unable to resolve fan RPM sysfs path"))
            }
        }
    }

    /// Build a new instance with PWM RPM file set
    pub(crate) fn with_rpm_file(self, path: &Path) -> anyhow::Result<Fan<PathBuf>> {
        Ok(Fan {
            pwm: self.pwm.with_rpm_file(path)?,
            thresholds: self.thresholds,
            speed: self.speed,
            startup: self.startup,
        })
    }
}

impl<T> Fan<T> {
    /// Compute PWM target value from speed and fan thresholds
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn speed_to_pwm_val(&self, speed: Speed) -> pwm::Value {
        if speed.is_zero() {
            pwm::Value::MIN
        } else {
            self.thresholds.max_stop
                + (f64::from(pwm::Value::MAX - self.thresholds.max_stop) * speed.0.get())
                    as pwm::Value
        }
    }

    /// Compute PWM target value directly from PWM percentage (for PID mode)
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn pwm_percent_to_pwm_val(&self, pwm_percent: f64) -> pwm::Value {
        (pwm_percent * f64::from(pwm::Value::MAX)) as pwm::Value
    }

    /// Apply startup boost logic to a calculated PWM value
    fn apply_startup_boost(&mut self, calculated_pwm: pwm::Value, _new_speed: Speed) -> pwm::Value {
        let is_startup_from_zero = self.speed.is_some_and(Speed::is_zero);
        let startup_time_remaining = self.startup
            .map(|s| STARTUP_DELAY.saturating_sub(Instant::now().duration_since(s)))
            .unwrap_or(Duration::ZERO);
        let is_in_startup_period = self.startup
            .is_some_and(|s| Instant::now().duration_since(s) < STARTUP_DELAY);
            
        log::debug!("Fan {self} startup status: from_zero={}, in_startup_period={}, time_remaining={:?}", 
                   is_startup_from_zero, is_in_startup_period, startup_time_remaining);
        
        if is_startup_from_zero {
            log::info!("Fan {self} startup from zero");
            self.startup = Some(Instant::now());
            let boosted = max(calculated_pwm, self.thresholds.min_start);
            log::debug!("Fan {self} startup boost: {calculated_pwm} -> {boosted}");
            boosted
        } else if is_in_startup_period {
            let boosted = max(calculated_pwm, self.thresholds.min_start);
            log::debug!("Fan {self} still in startup period: {calculated_pwm} -> {boosted} (remaining: {:?})", 
                       startup_time_remaining);
            boosted
        } else {
            log::debug!("Fan {self} normal operation: using calculated PWM {calculated_pwm}");
            calculated_pwm
        }
    }

    /// Set PWM mode to software control if needed
    fn ensure_software_mode(&mut self) -> anyhow::Result<()> {
        if let Some(prev_mode) = self.pwm.get_mode()? {
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
        }
        Ok(())
    }

    /// Set fan PWM directly from percentage (for PID mode)
    pub(crate) fn set_pwm_percent(&mut self, pwm_percent: f64) -> anyhow::Result<()> {
        log::debug!("Fan {self} set_pwm_percent called with pwm_percent: {:.3} ({:.1}%)", pwm_percent, pwm_percent * 100.0);
        
        // Convert PWM percentage to fake speed for tracking purposes
        let equivalent_speed = Speed::try_from(pwm_percent).map_err(|e| anyhow::anyhow!("Invalid PWM percentage: {}", e))?;
        
        if self.speed == Some(equivalent_speed) {
            log::trace!("Fan {self} PWM unchanged: {:.1}%", pwm_percent * 100.0);
            return Ok(());
        }

        self.ensure_software_mode()?;
        
        let calculated_pwm = self.pwm_percent_to_pwm_val(pwm_percent);
        log::debug!("Fan {self} calculated PWM from {:.1}% PWM: {}", pwm_percent * 100.0, calculated_pwm);
        
        let final_pwm = self.apply_startup_boost(calculated_pwm, equivalent_speed);
        
        log::debug!("Fan {self} final PWM value: {final_pwm} ({:.1}%)", 
                   final_pwm as f64 / 255.0 * 100.0);
        
        self.pwm.set(final_pwm)?;
        log::info!("Fan {self} PWM set to {:.1}%, PWM: {final_pwm}", pwm_percent * 100.0);
        self.speed = Some(equivalent_speed);
        
        Ok(())
    }

    /// Set fan speed
    pub(crate) fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        log::debug!("Fan {self} set_speed called with speed: {speed} ({:.3})", speed.0.get());
        
        if self.speed == Some(speed) {
            log::trace!("Fan {self} speed unchanged: {speed}");
            return Ok(());
        }

        self.ensure_software_mode()?;
        
        log::debug!("Fan {self} thresholds: min_start={}, max_stop={}", 
                   self.thresholds.min_start, self.thresholds.max_stop);
        
        let calculated_pwm = self.speed_to_pwm_val(speed);
        log::debug!("Fan {self} calculated PWM from speed {speed}: {calculated_pwm}");
        
        let final_pwm = self.apply_startup_boost(calculated_pwm, speed);
        
        log::debug!("Fan {self} final PWM value: {final_pwm} ({:.1}%)", 
                   final_pwm as f64 / 255.0 * 100.0);
        
        self.pwm.set(final_pwm)?;
        log::info!("Fan {self} speed set to {speed}, PWM: {final_pwm}");
        self.speed = Some(speed);
        
        Ok(())
    }
}

impl Fan<PathBuf> {
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
                #[expect(clippy::match_same_arms)]
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
    pub(crate) fn test(&mut self) -> anyhow::Result<Thresholds> {
        self.set_speed(1.0.try_into()?)?;
        self.wait_stable(SpeedChange::Increasing)?;
        anyhow::ensure!(self.is_moving()?, "Fan is not moving at maximum speed");

        let mut max_stop = 0;
        for pwm_val in (0..=pwm::Value::MAX).rev().step_by(5) {
            self.set_speed((f64::from(pwm_val) / f64::from(pwm::Value::MAX)).try_into()?)?;
            self.wait_stable(SpeedChange::Decreasing)?;
            if !self.is_moving()? {
                max_stop = pwm_val;
                break;
            }
        }
        anyhow::ensure!(!self.is_moving()?, "Fan still moves at minimum speed");

        let mut min_start = 0;
        for pwm_val in (0..=u8::MAX).step_by(5) {
            self.set_speed((f64::from(pwm_val) / f64::from(pwm::Value::MAX)).try_into()?)?;
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
pub(crate) fn target_speed(temp: Temp, temp_range: &Range<Temp>, min_speed: Speed) -> Speed {
    if temp_range.contains(&temp) {
        #[expect(clippy::unwrap_used)]
        let s = Speed::try_from((temp - temp_range.start) / (temp_range.end - temp_range.start))
            .unwrap();
        max(min_speed, s)
    } else if temp < temp_range.start {
        min_speed
    } else {
        #[expect(clippy::unwrap_used)]
        1.0.try_into().unwrap()
    }
}

/// Compute target fan speed for target temperature mode using real PID control
pub(crate) fn target_speed_with_pid(
    pid_controller: &mut pid::Pid<f64>,
    avg_temp: Temp,
    min_speed: Speed,
) -> Speed {
    // Get PID output as speed value (0.0-1.0)
    let pid_speed_output = pid_controller.next_control_output(avg_temp).output;
    
    // Clamp to valid speed range [min_speed, 1.0]
    let clamped_speed = pid_speed_output.max(min_speed.0.get()).min(1.0);
    
    // Scale to PWM range for debugging/logging
    let pwm_equiv = (clamped_speed * 255.0) as u8;
    let min_pwm = (min_speed.0.get() * 255.0) as u8;
    
    log::debug!(
        "PID: raw_output={:.3}, clamped_speed={:.3}, pwm_equiv={}, min_pwm={}", 
        pid_speed_output, clamped_speed, pwm_equiv, min_pwm
    );
    
    #[expect(clippy::unwrap_used)]
    Speed::try_from(clamped_speed).unwrap()
}

#[cfg(test)]
mod tests {

    use std::io::Write as _;

    use self::pwm::tests::{FakePwm, assert_file_content};
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
                Speed::try_from(0.2).unwrap()
            ),
            Speed::try_from(0.5).unwrap()
        );
        assert_eq!(
            target_speed(
                40.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.2).unwrap()
            ),
            Speed::try_from(0.2).unwrap()
        );
        assert_eq!(
            target_speed(
                35.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.2).unwrap()
            ),
            Speed::try_from(0.2).unwrap()
        );
        assert_eq!(
            target_speed(
                40.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.0).unwrap()
            ),
            Speed::try_from(0.0).unwrap()
        );
        assert_eq!(
            target_speed(
                35.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.0).unwrap()
            ),
            Speed::try_from(0.0).unwrap()
        );
        assert_eq!(
            target_speed(
                50.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.2).unwrap()
            ),
            Speed::try_from(1.0).unwrap()
        );
        assert_eq!(
            target_speed(
                55.0,
                &Range {
                    start: 40.0,
                    end: 50.0
                },
                Speed::try_from(0.2).unwrap()
            ),
            Speed::try_from(1.0).unwrap()
        );
    }

    #[test]
    fn test_target_speed_with_pid() {
        let min_speed = Speed::try_from(0.2).unwrap();
        let mut pid = pid::Pid::new(30.0, 1.0); // target 30°C, speed output range
        pid.p(-0.1, 1.0);  // negative for cooling system
        pid.i(-0.05, 1.0); // negative for cooling system
        pid.d(-0.01, 1.0); // negative for cooling system
        
        // Test that function returns valid speed values
        let speed = target_speed_with_pid(&mut pid, 25.0, min_speed);
        assert!(speed.0.get() >= min_speed.0.get());
        assert!(speed.0.get() <= 1.0);
        
        let speed = target_speed_with_pid(&mut pid, 35.0, min_speed);
        assert!(speed.0.get() >= min_speed.0.get());
        assert!(speed.0.get() <= 1.0);
    }

    #[test]
    fn test_set_speed() {
        let mut fake_pwm = FakePwm::new();
        let mut fan = Fan::new(&PwmSettings {
            filepath: fake_pwm.pwm_path.clone(),
            thresholds: Thresholds {
                min_start: 200,
                max_stop: 100,
            },
        })
        .unwrap();

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.0.try_into().unwrap()).unwrap();
        assert_eq!(fan.startup, None);
        assert_file_content(&mut fake_pwm.val_file_read, "0\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.01.try_into().unwrap()).unwrap();
        assert!(fan.startup.is_some());
        assert_file_content(&mut fake_pwm.val_file_read, "200\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.5.try_into().unwrap()).unwrap();
        assert!(fan.startup.is_some());
        assert_file_content(&mut fake_pwm.val_file_read, "200\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.9.try_into().unwrap()).unwrap();
        assert!(fan.startup.is_some());
        assert_file_content(&mut fake_pwm.val_file_read, "239\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(1.0.try_into().unwrap()).unwrap();
        assert!(fan.startup.is_some());
        assert_file_content(&mut fake_pwm.val_file_read, "255\n");

        fan.startup = None;

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.5.try_into().unwrap()).unwrap();
        assert_eq!(fan.startup, None);
        assert_file_content(&mut fake_pwm.val_file_read, "177\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.01.try_into().unwrap()).unwrap();
        assert_eq!(fan.startup, None);
        assert_file_content(&mut fake_pwm.val_file_read, "101\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.0.try_into().unwrap()).unwrap();
        assert_eq!(fan.startup, None);
        assert_file_content(&mut fake_pwm.val_file_read, "0\n");

        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        fan.set_speed(0.01.try_into().unwrap()).unwrap();
        assert!(fan.startup.is_some());
        assert_file_content(&mut fake_pwm.val_file_read, "200\n");
    }
}
