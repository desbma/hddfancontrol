//! Temperature probing

#![allow(dead_code)]

mod drivetemp;
mod hddtemp;
mod hdparm;
mod smartctl;

use crate::device::Drive;

/// Error returned when
#[derive(thiserror::Error, Debug)]
pub enum ProberError {
    /// Probing method is not supported by this drive on this system
    #[error("Temperature probing method unsupported: {0}")]
    Unsupported(String),
    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Temperature in Celcius
pub type Temp = f64;

/// A way to probe drive temperature
pub trait DriveTempProber: Sized {
    /// Build a new prober if supported for this device
    fn new(drive: &Drive) -> Result<Self, ProberError>;

    /// Get current drive temperature
    fn probe_temp(&mut self) -> anyhow::Result<Temp>;
}

/// Find first supported prober for a drive
pub fn prober(drive: &Drive) -> anyhow::Result<Option<drivetemp::Drivetemp>> {
    // TODO generic iteration over all probers
    match drivetemp::Drivetemp::new(drive) {
        Ok(p) => Ok(Some(p)),
        Err(ProberError::Unsupported(e)) => {
            log::info!("Drive {drive} does not support drivetemp: {e}");
            Ok(None)
        }
        Err(ProberError::Other(e)) => Err(e),
    }
}
