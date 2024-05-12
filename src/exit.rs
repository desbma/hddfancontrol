//! Exit hook to set PWM config

use crate::pwm;

/// Restore PWM config when dropped
#[allow(clippy::module_name_repetitions)]
pub struct ExitHook {
    /// Pwm and their config to restore
    pwms: Vec<(pwm::Pwm, pwm::State)>,
}

impl ExitHook {
    /// Build hook to restore current state on drop, or set max value
    pub fn new(pwms: Vec<pwm::Pwm>, restore: bool) -> anyhow::Result<Self> {
        Ok(Self {
            pwms: pwms
                .into_iter()
                .map(|p| -> anyhow::Result<(pwm::Pwm, pwm::State)> {
                    let state = if restore {
                        p.get_state()?
                    } else {
                        pwm::State {
                            value: pwm::Value::MAX,
                            mode: pwm::ControlMode::Software,
                        }
                    };
                    Ok((p, state))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

impl Drop for ExitHook {
    fn drop(&mut self) {
        for (pwm, state) in &mut self.pwms {
            let _ = pwm.set_state(state);
        }
    }
}
