//! Temperature probing

#![allow(dead_code)]

use crate::device::Drive;

/// Error returned when probing drive temperature
#[derive(thiserror::Error, Debug)]
enum ProbeError {
    /// Probing method is not supported by this drive on this system
    #[error("Temperature probing method unsupported: {0}")]
    Unsupported(anyhow::Error),
    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Temperature in Celcius
pub type Temp = f64;

/// A way to probe drive temperature
trait DriveTempProber {
    /// Get current drive temperature
    fn probe_temp(&mut self, drive: &Drive) -> Result<Temp, ProbeError>;
}
