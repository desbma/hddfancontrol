//! Fan control

#![allow(dead_code)]

use std::{fmt, path::Path};

use crate::pwm::{ControlMode, Pwm};

/// Stateful fan
pub struct Fan {
    /// Fan pwm
    pwm: Pwm,
    /// Current speed
    speed: Option<Speed>,
}

/// Fan speed as [0-255] value
#[derive(Copy, Clone, PartialEq)]
pub struct Speed(pub u8);

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:.2}%", f64::from(self.0) * 100.0 / f64::from(u8::MAX))
    }
}

impl Fan {
    /// Build a new fan from a PWM path
    pub fn new(pwm_path: &Path) -> anyhow::Result<Self> {
        let pwm = Pwm::new(pwm_path)?;
        Ok(Self { pwm, speed: None })
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
            let pwm_value = speed.0;
            self.pwm.set(pwm_value)?;
            log::info!("Fan {} speed set to {}", self.pwm, speed);
            self.speed = Some(speed);
        }
        Ok(())
    }
}
