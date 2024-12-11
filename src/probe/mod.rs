//! Temperature probing

mod drivetemp;
mod hddtemp;
mod hdparm;
mod smartctl;

use std::{
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
};

use crate::device::Drive;

/// Error returned when
#[derive(thiserror::Error, Debug)]
pub(crate) enum ProberError {
    /// Probing method is not supported by this drive on this system
    #[error("Temperature probing method unsupported: {0}")]
    Unsupported(String),
    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Temperature in Celcius
pub(crate) type Temp = f64;

/// A way to probe drive temperature
pub(crate) trait DriveTempProbeMethod: fmt::Display {
    /// Build a new prober if supported for this device
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DeviceTempProber>, ProberError>;

    /// Does prober supports probing spun down drive without waking it
    fn supports_probing_sleeping(&self) -> bool;
}

/// Device temperature prober
pub(crate) trait DeviceTempProber {
    /// Get current drive temperature
    fn probe_temp(&mut self) -> anyhow::Result<Temp>;
}

/// Find first supported prober for a drive
pub(crate) fn prober(
    drive: &Drive,
    hddtemp_daemon_port: u16,
) -> anyhow::Result<Option<(Box<dyn DeviceTempProber>, bool)>> {
    let methods: [Box<dyn DriveTempProbeMethod>; 6] = [
        Box::new(drivetemp::Method),
        Box::new(hdparm::Method),
        Box::new(smartctl::SctMethod),
        Box::new(hddtemp::DaemonMethod {
            addr: SocketAddrV4::new(Ipv4Addr::LOCALHOST, hddtemp_daemon_port),
        }),
        Box::new(hddtemp::InvocationMethod),
        Box::new(smartctl::AttribMethod),
    ];
    for method in methods {
        match method.prober(drive) {
            Ok(p) => {
                log::info!("Using probing method '{method}' for drive '{drive}'");
                let sqa = method.supports_probing_sleeping();
                return Ok(Some((p, sqa)));
            }
            Err(ProberError::Unsupported(e)) => {
                log::info!("Drive '{drive}' does not support probing method '{method}': {e}",);
            }
            Err(ProberError::Other(e)) => return Err(e),
        }
    }
    Ok(None)
}
