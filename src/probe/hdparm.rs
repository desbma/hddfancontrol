//! Hdparm temperature probing

use std::{
    fmt,
    io::BufRead,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::device::Drive;

use super::{DriveTempProbeMethod, DriveTempProber, ProberError, Temp};

/// Hdparm Hitachi/HGST temperature probing method
pub struct Method;

impl DriveTempProbeMethod for Method {
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DriveTempProber>, ProberError> {
        let mut prober = Prober {
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(Box::new(prober))
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "hdparm Hitachi/HGST")
    }
}

/// Hdparm Hitachi/HGST temperature prober
pub struct Prober {
    /// Device path in /dev/
    device: PathBuf,
}

impl DriveTempProber for Prober {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let output = Command::new("hdparm")
            .args([
                "-H",
                self.device
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stderr(Stdio::null())
            .env("LANG", "C")
            .output()?;
        let temp = output
            .stdout
            .lines()
            .map_while(Result::ok)
            .filter(|l| {
                l.trim_start()
                    .starts_with("drive temperature (celsius) is: ")
            })
            .find_map(|l| {
                l.split_ascii_whitespace()
                    .next_back()
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse hdparm temp output"))?
            .parse()?;
        Ok(temp)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use float_cmp::approx_eq;

    use super::*;

    use crate::tests::BinaryMock;

    #[serial_test::serial]
    #[test]
    fn test_hdparm_probe_temp() {
        let _hdparm = BinaryMock::new(
            "hdparm",
            "/dev/_sdX:\n  drive temperature (celsius) is:  30\n  drive temperature in range:  yes\n"
            .as_bytes(),
            &[],
            0,
        );
        let mut prober = Prober {
            device: PathBuf::from("/dev/_sdX"),
        };
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));
    }
}
