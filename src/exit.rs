//! Exit hook to set PWM config

use crate::pwm;

/// Restore PWM config when dropped
pub(crate) struct ExitHook<T> {
    /// Pwm and their config to restore
    pwms: Vec<(pwm::Pwm<T>, pwm::State)>,
}

impl<T> ExitHook<T> {
    /// Build hook to restore current state on drop, or set max value
    pub(crate) fn new(pwms: Vec<pwm::Pwm<T>>, restore: bool) -> anyhow::Result<Self> {
        Ok(Self {
            pwms: pwms
                .into_iter()
                .map(|p| -> anyhow::Result<(pwm::Pwm<_>, pwm::State)> {
                    let state = if restore {
                        p.get_state()?
                    } else {
                        pwm::State {
                            value: pwm::Value::MAX,
                            mode: p.get_mode()?.map(|_| pwm::ControlMode::Software),
                        }
                    };
                    Ok((p, state))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

impl<T> Drop for ExitHook<T> {
    fn drop(&mut self) {
        for (pwm, state) in &mut self.pwms {
            let _ = pwm.set_state(state);
        }
    }
}
