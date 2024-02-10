//! PWM fan code

// See https://docs.kernel.org/hwmon/pwm-fan.html

#![allow(dead_code)]

use std::{
    error::Error,
    fmt,
    fs::File,
    io::{Read, Write},
    os::linux::fs::MetadataExt,
    path::{Path, PathBuf},
    str,
    str::FromStr,
};

use nix::sys::stat;

/// PWM sysfs value
pub type Value = u8;

/// Stateless PWM control
pub struct Pwm {
    /// pwmX sysfs filepath
    val: PathBuf,
    /// fanX_input sysfs filepath
    rpm: PathBuf,
    /// pwmX_enable sysfs filepath
    mode: PathBuf,
    /// Kernel device name (different from PWM name)
    device: String,
    /// Index among driver
    num: usize,
}

/// PWM control modes
/// See <https://elixir.bootlin.com/linux/v6.7.2/source/drivers/hwmon/pwm-fan.c#L31>
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, int_enum::IntEnum, strum::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlMode {
    Off = 0,
    Software = 1,
    Hardware = 2,
}

impl Pwm {
    /// Build a PWM driver
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let path = Self::ensure_sysfs_file(path)?;
        let val_path_fname = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: {path:?}"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: {path:?}"))?;
        let num = val_path_fname
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()?;
        let rpm_path = Self::ensure_sysfs_file(&path.with_file_name(format!("fan{num}_input")))?;
        let mode_path =
            Self::ensure_sysfs_file(&path.with_file_name(format!("{val_path_fname}_enable")))?;
        let device = Self::ensure_sysfs_dir(&path.with_file_name("device"))
            .or_else(|_| Self::ensure_sysfs_dir(&path.with_file_name("driver")))?
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid device path for {path:?}"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid device name {path:?}"))?
            .to_owned();
        Ok(Self {
            val: path.clone(),
            rpm: rpm_path,
            mode: mode_path,
            device,
            num,
        })
    }

    /// Ensure path is a valid sysfs file path, and normalizes it
    fn ensure_sysfs_file(path: &Path) -> anyhow::Result<PathBuf> {
        let path = path.canonicalize()?;
        anyhow::ensure!(
            if cfg!(test) {
                path.is_file()
                    || (path.exists()
                        && (path.metadata()?.st_mode() & stat::SFlag::S_IFIFO.bits()) != 0)
            } else {
                path.is_file()
            },
            "{path:?} missing or not a file"
        );
        Ok(path)
    }

    /// Ensure path is a valid sysfs dir path, and normalizes it
    fn ensure_sysfs_dir(path: &Path) -> anyhow::Result<PathBuf> {
        let path = path.canonicalize()?;
        anyhow::ensure!(path.is_dir(), "{path:?} missing or not a directory");
        Ok(path)
    }

    /// Write integer value to path
    fn write_value<T>(path: &Path, val: T) -> anyhow::Result<()>
    where
        T: fmt::Display,
    {
        let mut f = File::create(path)?;
        f.write_all(format!("{val}\n").as_bytes())?;
        Ok(())
    }

    /// Read integer value from path
    fn read_value<T>(path: &Path) -> anyhow::Result<T>
    where
        T: FromStr + PartialEq + Copy,
        <T as FromStr>::Err: Error + Send + Sync,
        <T as FromStr>::Err: 'static,
    {
        let mut file = File::open(path)?;
        let mut buf = [0; 16];
        let count = file.read(&mut buf)?;
        let s = str::from_utf8(&buf[..count])?.trim_end();
        Ok(s.parse::<T>()?)
    }

    /// Set PWM value
    pub fn set(&self, val: Value) -> anyhow::Result<()> {
        Self::write_value(&self.val, val)
    }

    /// Get PWM value
    pub fn get(&self) -> anyhow::Result<Value> {
        Self::read_value(&self.val)
    }

    /// Get fan RPM value
    pub fn get_rpm(&self) -> anyhow::Result<u32> {
        Self::read_value(&self.rpm)
    }

    /// Get PWM control mode
    pub fn get_mode(&self) -> anyhow::Result<ControlMode> {
        Self::read_value::<u8>(&self.mode)?
            .try_into()
            .map_err(|v| anyhow::anyhow!("Unexpected mode: {v}"))
    }

    /// Set PWM control mode
    pub fn set_mode(&self, mode: ControlMode) -> anyhow::Result<()> {
        Self::write_value(&self.mode, mode as u8)
    }
}

impl fmt::Display for Pwm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.device, self.num)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        fs::{create_dir, File, OpenOptions},
        io::Read,
        os::unix::{fs::symlink, prelude::OpenOptionsExt},
        str,
    };

    use nix::{libc::O_NONBLOCK, sys::stat, unistd::mkfifo};
    use tempfile::TempDir;

    use super::*;

    struct FakePwm {
        dir: TempDir,
        pwm_path: PathBuf,
        val_file_read: File,
        val_file_write: File,
        rpm_file_read: File,
        rpm_file_write: File,
        mode_file_read: File,
        mode_file_write: File,
    }

    impl FakePwm {
        /// Setup fake test PWM paths
        fn new() -> Self {
            let dir = TempDir::new().unwrap();

            let pwm_path = dir.path().join("pwm2");
            mkfifo(&pwm_path, stat::Mode::from_bits(0o600).unwrap()).unwrap();
            let val_file_read = OpenOptions::new()
                .read(true)
                .custom_flags(O_NONBLOCK)
                .open(&pwm_path)
                .unwrap();
            let val_file_write = File::create(&pwm_path).unwrap();

            let rpm_path = dir.path().join("fan2_input");
            mkfifo(&rpm_path, stat::Mode::from_bits(0o600).unwrap()).unwrap();
            let rpm_file_read = OpenOptions::new()
                .read(true)
                .custom_flags(O_NONBLOCK)
                .open(&rpm_path)
                .unwrap();
            let rpm_file_write = File::create(&rpm_path).unwrap();

            let mode_path = dir.path().join("pwm2_enable");
            mkfifo(&mode_path, stat::Mode::from_bits(0o600).unwrap()).unwrap();
            let mode_file_read = OpenOptions::new()
                .read(true)
                .custom_flags(O_NONBLOCK)
                .open(&mode_path)
                .unwrap();
            let mode_file_write = File::create(&mode_path).unwrap();

            let device_path = dir.path().join("device_name");
            create_dir(&device_path).unwrap();
            let device_link = dir.path().join("device");
            symlink(device_path, device_link).unwrap();

            Self {
                dir,
                pwm_path,
                val_file_read,
                val_file_write,
                rpm_file_read,
                rpm_file_write,
                mode_file_read,
                mode_file_write,
            }
        }
    }

    fn assert_file_content(file: &mut File, content: &str) {
        let mut buf = [0; 16];
        let count = file.read(&mut buf).unwrap();
        let s = str::from_utf8(&buf[..count]).unwrap();
        assert_eq!(s, content);
    }

    #[test]
    fn test_set() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        pwm.set(123).unwrap();
        assert_file_content(&mut fake_pwm.val_file_read, "123\n");
    }

    #[test]
    fn test_get() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        fake_pwm.val_file_write.write_all(b"124\n").unwrap();
        assert_eq!(pwm.get().unwrap(), 124);
    }

    #[test]
    fn test_get_rpm() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        fake_pwm.rpm_file_write.write_all(b"1234\n").unwrap();
        assert_eq!(pwm.get_rpm().unwrap(), 1234);
    }

    #[test]
    fn test_get_mode() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        fake_pwm.mode_file_write.write_all(b"0\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap(), ControlMode::Off);
        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap(), ControlMode::Software);
        fake_pwm.mode_file_write.write_all(b"2\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap(), ControlMode::Hardware);
        fake_pwm.mode_file_write.write_all(b"3\n").unwrap();
        assert!(pwm.get_mode().is_err());
    }

    #[test]
    fn test_set_mode() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        pwm.set_mode(ControlMode::Off).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "0\n");
        pwm.set_mode(ControlMode::Software).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "1\n");
        pwm.set_mode(ControlMode::Hardware).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "2\n");
    }

    #[test]
    fn test_display() {
        let fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        assert_eq!(pwm.to_string().as_str(), "device_name/2");
    }
}
