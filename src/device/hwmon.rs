//! Hwmon probe

use core::fmt;
use std::{
    cmp::{max, min},
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

use crate::{
    probe::{DeviceTempProber, Temp},
    sysfs::{ensure_sysfs_dir, ensure_sysfs_file},
};

/// A linux whmon temp probe
pub(crate) struct Hwmon {
    /// Sysfs temperature probe path
    input_path: PathBuf,
    /// Kernel device name
    device: String,
    /// Probe index
    num: usize,
}

impl fmt::Display for Hwmon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.device, self.num)
    }
}

impl Hwmon {
    /// Build a new prober
    pub(crate) fn new(input_path: &Path) -> anyhow::Result<Self> {
        let device = ensure_sysfs_dir(&input_path.with_file_name("device"))
            .or_else(|_| ensure_sysfs_dir(&input_path.with_file_name("driver")))
            .context("Failed to get path for device/driver")?
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid device path for {input_path:?}"))?
            .to_owned();
        let num = ensure_sysfs_file(input_path)?
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid device path for {input_path:?}"))?
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse::<usize>()
            .with_context(|| format!("Failed to extract number from {input_path:?}"))?;
        Ok(Self {
            input_path: input_path.to_owned(),
            device,
            num,
        })
    }

    /// Get default temperature range
    pub(crate) fn default_range(&self) -> anyhow::Result<Range<Temp>> {
        let sysfs_dir = self
            .input_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid probe path {:?}", self.input_path))?;
        let sensor_num: u32 = self
            .input_path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid probe path {:?}", self.input_path))?
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()?;
        // Try to get crit and max temp
        let crit_filepath = sysfs_dir.join(format!("temp{sensor_num}_crit"));
        let crit_temp_milli = Self::read_sysfs_temp_milli(&crit_filepath)?;
        let max_filepath = sysfs_dir.join(format!("temp{sensor_num}_max"));
        let max_temp_milli = Self::read_sysfs_temp_milli(&max_filepath).unwrap_or_else(|_| {
            // Default to crit - 20 if we have no max temp
            crit_temp_milli - 1000 * 20
        });
        // Ensure they are in the correct order...
        let max_temp = f64::from(min(max_temp_milli, crit_temp_milli)) / 1000.0;
        let crit_temp = f64::from(max(max_temp_milli, crit_temp_milli)) / 1000.0;
        // Set range max as max minus a security margin, which is the difference between max and crit
        // The rationale is that this gap will be larger for devices with a large operating range, and vice versa
        Ok(Range {
            start: 30.0,
            end: max_temp - (crit_temp - max_temp),
        })
    }

    /// Read a sysfs temp probe
    fn read_sysfs_temp(path: &Path) -> anyhow::Result<Temp> {
        Ok(f64::from(Self::read_sysfs_temp_milli(path)?) / 1000.0)
    }

    /// Read a sysfs temp probe
    fn read_sysfs_temp_milli(path: &Path) -> anyhow::Result<u32> {
        Ok(fs::read_to_string(path)?.trim_end().parse()?)
    }
}

impl DeviceTempProber for Hwmon {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        Self::read_sysfs_temp(&self.input_path)
    }
}
