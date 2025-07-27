//! sg_logs temperature probing

use std::{
    fmt,
    io::BufRead as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use super::{DeviceTempProber, DriveTempProbeMethod, ProberError, Temp};
use crate::device::Drive;

/// sg_logs temperature probing method
pub(crate) struct Method;

impl DriveTempProbeMethod for Method {
    type Prober = Prober;

    fn prober(&self, drive: &Drive) -> Result<Prober, ProberError> {
        let mut prober = Prober {
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(prober)
    }

    fn supports_probing_sleeping(&self) -> bool {
        true
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "sg_logs temperature log page")
    }
}

/// sg_logs temperature prober
pub(crate) struct Prober {
    /// Device path in /dev/
    device: PathBuf,
}

impl DeviceTempProber for Prober {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let output = Command::new("sg_logs")
            .args([
                "-p",
                "0xd", // Temperature log page
                self.device
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stdin(Stdio::null())
            .env("LANG", "C")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "sg_logs failed with code {}",
            output.status
        );
        let lines: Vec<_> = output
            .stdout
            .lines()
            .chain(output.stderr.lines())
            .collect::<Result<_, _>>()?;

        let temp = lines
            .iter()
            .filter(|l| l.trim_start().starts_with("Current temperature ="))
            .find_map(|l| {
                l.split_ascii_whitespace()
                    .nth(3) // "Current" "temperature" "=" "<value>"
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse sg_logs temp output"))?
            .parse()?;
        Ok(temp)
    }
}

#[expect(clippy::shadow_unrelated)]
#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;

    use super::*;
    use crate::tests::BinaryMock;

    #[serial_test::serial]
    #[test]
    fn test_sglogs_probe_temp() {
        let mut prober = Prober {
            device: PathBuf::from("/dev/_sdX"),
        };

        // Valid output with 32 C
        let _sglogs = BinaryMock::new(
            "sg_logs",
            b"    WDC       WUH721818AL5200   US05\nTemperature log page  [0xd]\n  Current temperature = 32 C\n  Reference temperature = 50 C\n",
            &[],
            0,
        );
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 32.0));

        // Missing temperature line
        let _sglogs = BinaryMock::new(
            "sg_logs",
            b"    WDC       WUH721818AL5200   US05\nTemperature log page  [0xd]\n",
            &[],
            0,
        );
        assert!(prober.probe_temp().is_err());

        // Non-zero exit code
        let _sglogs = BinaryMock::new(
            "sg_logs",
            b"",
            &[],
            1,
        );
        assert!(prober.probe_temp().is_err());
    }
}
