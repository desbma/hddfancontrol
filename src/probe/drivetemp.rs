//! Drivetemp native kernel temperature probing
//! See <https://docs.kernel.org/hwmon/drivetemp.html>

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use super::{DeviceTempProber, Drive, DriveTempProbeMethod, ProberError, Temp};

/// Drivetemp native kernel temperature probing method
pub(crate) struct Method;

impl DriveTempProbeMethod for Method {
    fn prober(&self, drive: &Drive) -> Result<Box<dyn DeviceTempProber>, ProberError> {
        #[expect(clippy::unwrap_used)] // At this point we already checked it is a valid device
        let drive_name = drive.dev_path.file_name().unwrap();
        let hwmon_dir = Path::new("/sys/block/")
            .join(drive_name)
            .join("../../hwmon");
        if !hwmon_dir.is_dir() {
            return Err(ProberError::Unsupported(format!(
                "{hwmon_dir:?} does not exist"
            )));
        }
        for hwmon_subdir_entry in fs::read_dir(&hwmon_dir)
            .map_err(|e| ProberError::Other(e.into()))?
            .map_while(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        {
            let hwmon_subdir = hwmon_subdir_entry.path();
            let name_file = hwmon_subdir.join("name");
            let name = fs::read_to_string(&name_file)
                .map_err(|e| ProberError::Other(e.into()))?
                .trim_end()
                .to_owned();
            if name == "drivetemp" {
                let input_path = hwmon_subdir.join("temp1_input");
                if !input_path.is_file() {
                    return Err(ProberError::Other(anyhow::anyhow!(
                        "{input_path:?} does not exist"
                    )));
                }
                return Ok(Box::new(Prober { input_path }));
            }
        }
        Err(ProberError::Unsupported(format!(
            "No drivetemp hwmon found in {hwmon_dir:?}"
        )))
    }

    fn supports_probing_sleeping(&self) -> bool {
        true
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "native Linux drivetemp")
    }
}

/// Drivetemp kernel temperature prober
pub(crate) struct Prober {
    /// Sysfs file, ie `temp1_input`
    input_path: PathBuf,
}

impl DeviceTempProber for Prober {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        Ok(f64::from(
            fs::read_to_string(&self.input_path)?
                .trim_end()
                .parse::<u32>()?,
        ) / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use float_cmp::approx_eq;

    use super::*;

    #[test]
    fn test_probe_temp() {
        let mut input_file = tempfile::NamedTempFile::new().unwrap();
        let mut prober = Prober {
            input_path: input_file.path().to_owned(),
        };
        input_file.write_all("54321\n".as_bytes()).unwrap();
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 54.321));
    }
}
