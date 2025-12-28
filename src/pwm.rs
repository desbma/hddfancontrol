//! PWM fan code

// See https://docs.kernel.org/hwmon/pwm-fan.html

use std::{
    fmt,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context as _;
use backon::{BackoffBuilder as _, BlockingRetryable as _};

use crate::sysfs::{ensure_sysfs_dir, ensure_sysfs_file, read_value, write_value};

/// PWM sysfs value
pub(crate) type Value = u8;

/// Stateless PWM control
/// T is the type of RPM file path
#[derive(Clone)]
pub(crate) struct Pwm<T> {
    /// pwmX sysfs filepath
    val: PathBuf,
    /// `fanX_input` sysfs filepath
    rpm: T,
    /// `pwmX_enable` sysfs filepath
    mode: Option<PathBuf>,
    /// Kernel device name (different from PWM name)
    device: String,
    /// Index among driver
    num: usize,
}

/// PWM control modes
/// See <https://elixir.bootlin.com/linux/v6.7.2/source/drivers/hwmon/pwm-fan.c#L31>
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, strum::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum ControlMode {
    Off = 0,
    Software = 1,
    /// Other value, may be driver-specific, so don't assume any meaning
    Other(u8),
}

impl From<u8> for ControlMode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::Software,
            v => Self::Other(v),
        }
    }
}

impl From<ControlMode> for u8 {
    fn from(val: ControlMode) -> Self {
        match val {
            ControlMode::Off => 0,
            ControlMode::Software => 1,
            ControlMode::Other(v) => v,
        }
    }
}

/// Pwm state used to restore initial state
pub(crate) struct State {
    /// Original PWM value
    pub value: Value,
    /// Original PWM control mode
    pub mode: Option<ControlMode>,
}

impl Pwm<()> {
    /// Build a PWM driver
    pub(crate) fn new(path: &Path) -> anyhow::Result<Self> {
        // At boot sometimes the PWM is not immediately available, so retry a few times if not found,
        // with increasing delay
        let path = (|| ensure_sysfs_file(path))
            .retry(
                backon::ExponentialBuilder::default()
                    .with_factor(1.5)
                    .with_min_delay(Duration::from_millis(10))
                    .with_max_delay(Duration::from_secs(1))
                    .without_max_times()
                    .build(),
            )
            .when(|err| {
                err.downcast_ref::<io::Error>()
                    .is_some_and(|ioe| ioe.kind() == ErrorKind::NotFound)
            })
            .notify(|_err, dur| log::warn!("{path:?} does not exist, retrying in {dur:?}"))
            .call()?;

        let val_path_fname = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid path: {path:?}"))?;
        let num = val_path_fname
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()
            .with_context(|| {
                format!("Unable to extract pwm number from file name {val_path_fname:?}")
            })?;
        let mode_path =
            ensure_sysfs_file(&path.with_file_name(format!("{val_path_fname}_enable"))).ok();
        let device = ensure_sysfs_dir(&path.with_file_name("device"))
            .or_else(|_| ensure_sysfs_dir(&path.with_file_name("driver")))
            .context("Failed to get path for device/driver")?
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid device path for {path:?}"))?
            .to_owned();
        Ok(Self {
            val: path.clone(),
            rpm: (),
            mode: mode_path,
            device,
            num,
        })
    }

    /// Build a new PWM with RPM file path set
    pub(crate) fn with_rpm_file(self, rpm_path: &Path) -> anyhow::Result<Pwm<PathBuf>> {
        Ok(Pwm {
            val: self.val,
            rpm: ensure_sysfs_file(rpm_path)?,
            mode: self.mode,
            device: self.device,
            num: self.num,
        })
    }

    /// Get sysfs directory
    pub(crate) fn sysfs_dir(&self) -> &Path {
        #[expect(clippy::unwrap_used)]
        self.val.parent().unwrap()
    }
}

impl<T> Pwm<T> {
    /// Set PWM value
    pub(crate) fn set(&self, val: Value) -> anyhow::Result<()> {
        log::trace!("Set PWM {self} to {val}");
        write_value(&self.val, val).with_context(|| format!("Failed to write to {:?}", self.val))
    }

    /// Get PWM value
    pub(crate) fn get(&self) -> anyhow::Result<Value> {
        read_value(&self.val).with_context(|| format!("Failed to read from {:?}", self.val))
    }

    /// Get PWM control mode
    pub(crate) fn get_mode(&self) -> anyhow::Result<Option<ControlMode>> {
        if let Some(mode) = self.mode.as_ref() {
            Ok(Some(
                read_value::<u8>(mode)
                    .with_context(|| format!("Failed to read from {mode:?}"))?
                    .into(),
            ))
        } else {
            Ok(None)
        }
    }

    /// Set PWM control mode
    pub(crate) fn set_mode(&self, mode: ControlMode) -> anyhow::Result<()> {
        if let Some(mode_path) = self.mode.as_ref() {
            write_value::<u8>(mode_path, mode.into())
                .with_context(|| format!("Failed to write to {mode_path:?}"))
        } else {
            Ok(())
        }
    }

    /// Get current state
    pub(crate) fn get_state(&self) -> anyhow::Result<State> {
        Ok(State {
            value: self.get()?,
            mode: self.get_mode()?,
        })
    }

    /// Set state
    pub(crate) fn set_state(&self, state: &State) -> anyhow::Result<()> {
        self.set(state.value)?;
        if let Some(mode) = state.mode {
            self.set_mode(mode)?;
        }
        Ok(())
    }
}

impl Pwm<PathBuf> {
    /// Get fan RPM value
    pub(crate) fn get_rpm(&self) -> anyhow::Result<u32> {
        read_value(&self.rpm).with_context(|| format!("Failed to read from {:?}", self.rpm))
    }
}

impl<T> fmt::Display for Pwm<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.device, self.num)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        fs::{File, OpenOptions, create_dir},
        io::Read as _,
        os::unix::{fs::symlink, prelude::OpenOptionsExt as _},
        str,
    };

    use io::Write as _;
    use nix::{libc::O_NONBLOCK, sys::stat, unistd::mkfifo};
    use tempfile::TempDir;

    use super::*;

    pub(crate) struct FakePwm {
        _dir: TempDir,
        pub pwm_path: PathBuf,
        pub val_file_read: File,
        val_file_write: File,
        pub rpm_path: PathBuf,
        _rpm_file_read: File,
        rpm_file_write: File,
        mode_file_read: File,
        pub mode_file_write: File,
    }

    impl FakePwm {
        /// Setup fake test PWM paths
        pub(crate) fn new() -> Self {
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
                _dir: dir,
                pwm_path,
                val_file_read,
                val_file_write,
                rpm_path,
                _rpm_file_read: rpm_file_read,
                rpm_file_write,
                mode_file_read,
                mode_file_write,
            }
        }
    }

    pub(crate) fn assert_file_content(file: &mut File, content: &str) {
        let mut buf = [0; 16];
        let count = file.read(&mut buf).unwrap();
        let s = str::from_utf8(&buf[..count]).unwrap();
        assert_eq!(s, content);
    }

    #[test]
    fn set() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        pwm.set(123).unwrap();
        assert_file_content(&mut fake_pwm.val_file_read, "123\n");
    }

    #[test]
    fn get() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        fake_pwm.val_file_write.write_all(b"124\n").unwrap();
        assert_eq!(pwm.get().unwrap(), 124);
    }

    #[test]
    fn get_rpm() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path)
            .unwrap()
            .with_rpm_file(&fake_pwm.rpm_path)
            .unwrap();
        fake_pwm.rpm_file_write.write_all(b"1234\n").unwrap();
        assert_eq!(pwm.get_rpm().unwrap(), 1234);
    }

    #[test]
    fn get_mode() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        fake_pwm.mode_file_write.write_all(b"0\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap().unwrap(), ControlMode::Off);
        fake_pwm.mode_file_write.write_all(b"1\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap().unwrap(), ControlMode::Software);
        fake_pwm.mode_file_write.write_all(b"2\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap().unwrap(), ControlMode::Other(2));
        fake_pwm.mode_file_write.write_all(b"3\n").unwrap();
        assert_eq!(pwm.get_mode().unwrap().unwrap(), ControlMode::Other(3));
    }

    #[test]
    fn set_mode() {
        let mut fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        pwm.set_mode(ControlMode::Off).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "0\n");
        pwm.set_mode(ControlMode::Software).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "1\n");
        pwm.set_mode(ControlMode::Other(2)).unwrap();
        assert_file_content(&mut fake_pwm.mode_file_read, "2\n");
    }

    #[test]
    fn display() {
        let fake_pwm = FakePwm::new();
        let pwm = Pwm::new(&fake_pwm.pwm_path).unwrap();
        assert_eq!(pwm.to_string().as_str(), "device_name/2");
    }
}
