//! Fan control

mod cmd_fan;
mod pwm_fan;

use std::{cmp::max, fmt, ops::Range};

pub(crate) use cmd_fan::CommandFan;
pub(crate) use pwm_fan::PwmFan;

use crate::{probe::Temp, pwm};

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

/// Fan speed as [0-1] value
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Speed(typed_floats::PositiveFinite<f64>);

impl Speed {
    /// Test if speed is null
    pub(crate) fn is_zero(self) -> bool {
        self.0.is_positive_zero()
    }
}

/// Speed conversion error
#[derive(thiserror::Error, Debug)]
pub(crate) enum SpeedConversionError {
    /// Value not in range
    #[error("Value not in range [0.0; 1.0]")]
    Range,
    /// Invalid number
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

/// Fan speed control
pub(crate) trait Fan: fmt::Display {
    /// Set fan speed
    fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()>;
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn convert_target_speed() {
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
}
