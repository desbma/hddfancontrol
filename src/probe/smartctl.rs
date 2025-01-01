//! Smartctl temperature probing

use std::{
    fmt,
    io::BufRead as _,
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use super::{DeviceTempProber, Drive, DriveTempProbeMethod, ProberError, Temp};

/// Smartctl SCT temperature probing method
pub(crate) struct SctMethod;

impl DriveTempProbeMethod for SctMethod {
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DeviceTempProber>, ProberError> {
        let mut prober = SctProber {
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

impl fmt::Display for SctMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "smartctl SCT")
    }
}

/// Smartctl SCT temperature prober
pub(crate) struct SctProber {
    /// Device path in /dev/
    device: PathBuf,
}

impl DeviceTempProber for SctProber {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let output = Command::new("smartctl")
            .args([
                "-l",
                "scttempsts",
                self.device
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .env("LANG", "C")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "smartctl failed with code {}",
            output.status
        );
        let temp = output
            .stdout
            .lines()
            .map_while(Result::ok)
            .filter(|l| l.starts_with("Current Temperature: "))
            .find_map(|l| {
                l.split_ascii_whitespace()
                    .rev()
                    .nth(1)
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse smartctl SCT temp output"))?
            .parse()?;
        Ok(temp)
    }
}

/// Smartctl SMART attribute temperature probing method
pub(crate) struct AttribMethod;

impl DriveTempProbeMethod for AttribMethod {
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DeviceTempProber>, ProberError> {
        let mut prober = AttribProber {
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(Box::new(prober))
    }

    fn supports_probing_sleeping(&self) -> bool {
        false
    }
}

impl fmt::Display for AttribMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "smartctl SMART attribute")
    }
}

/// Smartctl SMART attribute temperature prober
pub(crate) struct AttribProber {
    /// Device path in /dev/
    device: PathBuf,
}

/// SMART attribute log, as parsed from smartctl output
struct SmartAttribLog {
    /// Attribute id
    id: u16,
    /// Attribute name
    name: String,
    /// Attribute value
    value: u32,
}

impl FromStr for SmartAttribLog {
    type Err = &'static str;

    /// Parse log from smartctl -A output
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<_> = s.split_ascii_whitespace().collect();
        if tokens.len() < 10 {
            return Err("No enough columns");
        }
        Ok(Self {
            id: tokens[0]
                .parse()
                .map_err(|_| "Unable to parse attribute id")?,
            name: tokens[1].to_owned(),
            value: tokens[9]
                .parse()
                .map_err(|_| "Unable to parse attribute value")?,
        })
    }
}

impl SmartAttribLog {
    /// Get temp if this attribute has it, or None
    fn temp(&self) -> Option<Temp> {
        /// Known temp attributes
        const TEMP_ATTRIBS_ID_NAMES: [(u16, &str); 2] = [
            (194, "Temperature_Celsius"),
            (190, "Airflow_Temperature_Ce"),
        ];
        for attrib in TEMP_ATTRIBS_ID_NAMES {
            if (self.id == attrib.0) && (self.name == attrib.1) {
                return Some(Temp::from(self.value));
            }
        }
        None
    }
}

impl DeviceTempProber for AttribProber {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let output = Command::new("smartctl")
            .args([
                "-A",
                self.device
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .env("LANG", "C")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "smartctl failed with code {}",
            output.status
        );
        let temp = output
            .stdout
            .lines()
            .map_while(Result::ok)
            .find_map(|l| l.parse::<SmartAttribLog>().ok().and_then(|a| a.temp()))
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to parse smartctl attribute output, or no temp attribute")
            })?;
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
    fn test_sct_probe_temp() {
        let _smartctl = BinaryMock::new(
            "smartctl",
            "smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SCT Status Version:                  3
SCT Version (vendor specific):       258 (0x0102)
Device State:                        Active (0)
Current Temperature:                    30 Celsius
Power Cycle Min/Max Temperature:     18/40 Celsius
Lifetime    Min/Max Temperature:      0/56 Celsius
Under/Over Temperature Limit Count:   0/0
Vendor specific:
01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00

"
            .as_bytes(),
            &[],
            0,
        );
        let mut prober = SctProber {
            device: PathBuf::from("/dev/_sdX"),
        };
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));
    }

    #[serial_test::serial]
    #[test]
    fn test_attrib_probe_temp() {
        let _smartctl = BinaryMock::new(
            "smartctl",
            "smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SMART Attributes Data Structure revision number: 16
Vendor Specific SMART Attributes with Thresholds:
ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  1 Raw_Read_Error_Rate     0x000b   100   100   016    Pre-fail  Always       -       0
  2 Throughput_Performance  0x0005   136   136   054    Pre-fail  Offline      -       80
  3 Spin_Up_Time            0x0007   123   123   024    Pre-fail  Always       -       615 (Average 644)
  4 Start_Stop_Count        0x0012   100   100   000    Old_age   Always       -       540
  5 Reallocated_Sector_Ct   0x0033   100   100   005    Pre-fail  Always       -       0
  7 Seek_Error_Rate         0x000b   100   100   067    Pre-fail  Always       -       0
  8 Seek_Time_Performance   0x0005   124   124   020    Pre-fail  Offline      -       33
  9 Power_On_Hours          0x0012   100   100   000    Old_age   Always       -       1723
 10 Spin_Retry_Count        0x0013   100   100   060    Pre-fail  Always       -       0
 12 Power_Cycle_Count       0x0032   100   100   000    Old_age   Always       -       424
192 Power-Off_Retract_Count 0x0032   100   100   000    Old_age   Always       -       571
193 Load_Cycle_Count        0x0012   100   100   000    Old_age   Always       -       571
194 Temperature_Celsius     0x0002   171   171   000    Old_age   Always       -       35 (Min/Max 13/45)
196 Reallocated_Event_Count 0x0032   100   100   000    Old_age   Always       -       0
197 Current_Pending_Sector  0x0022   100   100   000    Old_age   Always       -       0
198 Offline_Uncorrectable   0x0008   100   100   000    Old_age   Offline      -       0
199 UDMA_CRC_Error_Count    0x000a   200   200   000    Old_age   Always       -       0

"
            .as_bytes(),
            &[],
            0,
        );
        let mut prober = AttribProber {
            device: PathBuf::from("/dev/_sdX"),
        };
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 35.0));
    }
}
