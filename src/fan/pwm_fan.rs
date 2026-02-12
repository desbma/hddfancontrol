//! PWM fan control

use std::{
    cmp::{Ordering, max},
    fmt,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::Context as _;

use super::{Fan, Speed, Thresholds};
use crate::{
    cl::PwmSettings,
    pwm::{self, ControlMode, Pwm},
};

/// Minimum duration to apply fan startup boost
const STARTUP_DELAY: Duration = Duration::from_secs(20);

/// Stateful PWM fan
pub(crate) struct PwmFan<T> {
    /// Fan pwm
    pwm: Pwm<T>,
    /// Pwm thresholds
    thresholds: Thresholds,
    /// Current speed
    speed: Option<Speed>,
    /// Startup ts
    startup: Option<Instant>,
}

impl<T> fmt::Display for PwmFan<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.pwm.fmt(f)
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

impl PwmFan<()> {
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
    pub(crate) fn with_rpm_file(self, path: &Path) -> anyhow::Result<PwmFan<PathBuf>> {
        Ok(PwmFan {
            pwm: self.pwm.with_rpm_file(path)?,
            thresholds: self.thresholds,
            speed: self.speed,
            startup: self.startup,
        })
    }
}

impl<T> PwmFan<T> {
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
}

impl<T> Fan for PwmFan<T> {
    fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        if self.speed == Some(speed) {
            log::trace!("Fan {self} speed unchanged: {speed}");
        } else {
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
            let pwm_value = self.speed_to_pwm_val(speed);
            let pwm_value = if self.speed.is_some_and(Speed::is_zero) {
                log::info!("Fan {self} startup");
                self.startup = Some(Instant::now());
                max(pwm_value, self.thresholds.min_start)
            } else if self
                .startup
                .is_some_and(|s| Instant::now().duration_since(s) < STARTUP_DELAY)
            {
                max(pwm_value, self.thresholds.min_start)
            } else {
                pwm_value
            };
            self.pwm.set(pwm_value)?;
            log::info!("Fan {self} speed set to {speed}");
            self.speed = Some(speed);
        }
        Ok(())
    }
}

impl PwmFan<PathBuf> {
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

#[cfg(test)]
mod tests {

    use std::io::Write as _;

    use super::*;
    use crate::pwm::tests::{FakePwm, assert_file_content};

    #[test]
    fn set_speed() {
        let mut fake_pwm = FakePwm::new();
        let mut fan = PwmFan::new(&PwmSettings {
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
