//! Hdparm temperature probing

use std::{
    fmt,
    io::BufRead as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use super::{DeviceTempProber, DriveTempProbeMethod, ProberError, Temp};
use crate::device::Drive;

/// Hdparm Hitachi/HGST temperature probing method
pub(crate) struct Method;

impl DriveTempProbeMethod for Method {
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DeviceTempProber>, ProberError> {
        let mut prober = Prober {
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(Box::new(prober))
    }

    fn supports_probing_sleeping(&self) -> bool {
        true
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "hdparm Hitachi/HGST")
    }
}

/// Hdparm Hitachi/HGST temperature prober
pub(crate) struct Prober {
    /// Device path in /dev/
    device: PathBuf,
}

impl DeviceTempProber for Prober {
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
        anyhow::ensure!(
            output.status.success(),
            "hdparm failed with code {}",
            output.status
        );
        let lines: Vec<_> = output.stdout.lines().collect::<Result<_, _>>()?;
        anyhow::ensure!(
            !lines
                .iter()
                .any(|l| l.starts_with("SG_IO: ") && l.contains("sense data")),
            "hdparm returned soft error",
        );
        let temp = lines
            .iter()
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
mod tests {
    use float_cmp::approx_eq;

    use super::*;
    use crate::tests::BinaryMock;

    #[serial_test::serial]
    #[test]
    fn test_hdparm_probe_temp() {
        let mut prober = Prober {
            device: PathBuf::from("/dev/_sdX"),
        };

        let _hdparm = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n  drive temperature (celsius) is:  30\n  drive temperature in range:  yes\n"
            .as_bytes(),
            &[],
            0,
        );
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));

        let _hdparm = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\nSG_IO: questionable sense data, results may be incorrect\n drive temperature (celsius) is: -18\n drive temperature in range: yes\n"
            .as_bytes(),
            &[],
            0,
        );
        assert!(prober.probe_temp().is_err());

        let _hdparm = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\nSG_IO: missing sense data, results may be incorrect\n drive temperature (celsius) is: -18\n drive temperature in range: yes\n"
            .as_bytes(),
            &[],
            0,
        );
        assert!(prober.probe_temp().is_err());

        let _hdparm = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdz:\nSG_IO: bad/missing sense data, sb[]: 70 00 05 00 00 00 00 0a 04 51 40 00 21 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00\n drive temperature (celsius) is: -18\n drive temperature in range: yes\n"
            .as_bytes(),
            &[],
            0,
        );
        assert!(prober.probe_temp().is_err());
    }
}
