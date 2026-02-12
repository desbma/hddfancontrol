//! External command fan control

use std::{
    ffi::{OsStr, OsString},
    fmt,
    process::{Command, Stdio},
};

use anyhow::Context as _;

use super::{Fan, Speed};

/// Fan controlled by an external command
pub(crate) struct CommandFan {
    /// Command path
    cmd: OsString,
}

impl CommandFan {
    /// Build a new command fan
    pub(crate) fn new(cmd: &OsStr) -> Self {
        Self {
            cmd: cmd.to_owned(),
        }
    }
}

impl fmt::Display for CommandFan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.cmd.to_string_lossy())
    }
}

impl Fan for CommandFan {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        let value = (speed.0.get() * 1000.0) as u16;
        log::debug!(
            "Setting fan speed with command: {} {}",
            self.cmd.display(),
            value
        );
        let status = Command::new(&self.cmd)
            .arg(value.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to run fan command {self}"))?;
        anyhow::ensure!(
            status.success(),
            "Fan command {self} failed with status {status}"
        );
        log::info!("Fan {self} speed set to {speed}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn set_speed_success() {
        let mut fan = CommandFan::new(OsStr::new("true"));
        fan.set_speed(0.5.try_into().unwrap()).unwrap();
    }

    #[test]
    fn set_speed_failure() {
        let mut fan = CommandFan::new(OsStr::new("false"));
        assert!(fan.set_speed(0.5.try_into().unwrap()).is_err());
    }

    #[test]
    fn display() {
        let fan = CommandFan::new(OsStr::new("/usr/bin/fan_set"));
        assert_eq!(fan.to_string(), "/usr/bin/fan_set");
    }
}
