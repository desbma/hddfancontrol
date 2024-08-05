//! PWM fan code

// See https://docs.kernel.org/hwmon/pwm-fan.html

use std::{
    fmt,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::Duration,
};

use backoff::ExponentialBackoffBuilder;

use crate::sysfs::{ensure_sysfs_dir, ensure_sysfs_file, read_value, write_value};

/// PWM sysfs value
pub(crate) type Value = u8;

/// Stateless PWM control
pub(crate) struct Pwm {
    /// pwmX sysfs filepath
    val: PathBuf,
    /// `fanX_input` sysfs filepath
    rpm: PathBuf,
    /// `pwmX_enable` sysfs filepath
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

/// Pwm state used to restore initial state
pub(crate) struct State {
    /// Original PWM value
    pub value: Value,
    /// Original PWM control mode
    pub mode: ControlMode,
}

impl Pwm {
    /// Build a PWM driver
    pub(crate) fn new(path: &Path) -> anyhow::Result<Self> {
        // At boot sometimes the PWM is not immediately available, so retry a few times if not found,
        // with increasing delay
        let retrier = ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(10))
            .with_randomization_factor(0.0)
            .with_multiplier(1.5)
            .with_max_interval(Duration::from_secs(1))
            .with_max_elapsed_time(Some(Duration::from_secs(10)))
            .build();
        let path = backoff::retry_notify(
            retrier,
            || match ensure_sysfs_file(path) {
                Ok(p) => Ok(p),
                Err(e)
                    if e.downcast_ref::<io::Error>()
                        .is_some_and(|ioe| ioe.kind() == ErrorKind::NotFound) =>
                {
                    Err(backoff::Error::transient(e))
                }
                Err(e) => Err(backoff::Error::permanent(e)),
            },
            |_e, d| log::warn!("{path:?} does not exist, retrying in {d:?}"),
        )
        .map_err(|e| match e {
            backoff::Error::Permanent(e) => e,
            backoff::Error::Transient { err, .. } => err,
        })?;

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
        let rpm_path = ensure_sysfs_file(&path.with_file_name(format!("fan{num}_input")))?;
        let mode_path =
            ensure_sysfs_file(&path.with_file_name(format!("{val_path_fname}_enable")))?;
        let device = ensure_sysfs_dir(&path.with_file_name("device"))
            .or_else(|_| ensure_sysfs_dir(&path.with_file_name("driver")))?
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

    /// Set PWM value
    pub(crate) fn set(&self, val: Value) -> anyhow::Result<()> {
        log::trace!("Set PWM {self} to {val}");
        write_value(&self.val, val)
    }

    /// Get PWM value
    pub(crate) fn get(&self) -> anyhow::Result<Value> {
        read_value(&self.val)
    }

    /// Get fan RPM value
    pub(crate) fn get_rpm(&self) -> anyhow::Result<u32> {
        read_value(&self.rpm)
    }

    /// Get PWM control mode
    pub(crate) fn get_mode(&self) -> anyhow::Result<ControlMode> {
        read_value::<u8>(&self.mode)?
            .try_into()
            .map_err(|v| anyhow::anyhow!("Unexpected mode: {v}"))
    }

    /// Set PWM control mode
    pub(crate) fn set_mode(&self, mode: ControlMode) -> anyhow::Result<()> {
        write_value(&self.mode, mode as u8)
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
        self.set_mode(state.mode)?;
        Ok(())
    }
}

impl fmt::Display for Pwm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.device, self.num)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        fs::{create_dir, File, OpenOptions},
        io::Read,
        os::unix::{fs::symlink, prelude::OpenOptionsExt},
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
