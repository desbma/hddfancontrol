//! Block device drive

use std::{
    ffi::OsStr,
    fmt, fs,
    io::BufRead as _,
    os::unix::prelude::FileTypeExt as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Drive runtime state
#[derive(strum::EnumString, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum State {
    /// Suspended by kernel power management
    PmSuspended,
    /// Active/idle
    #[strum(serialize = "active/idle")]
    ActiveIdle,
    /// Standby
    Standby,
    /// Sleeping (power saving mode)
    Sleeping,
    /// Error occured while querying drive state
    Unknown,
}

/// How to probe for drive state
#[derive(strum::Display)]
#[strum(serialize_all = "lowercase")]
enum StateProbingMethod {
    /// Use `hdparm`
    Hdparm,
    /// Use `sdparm`
    Sdparm,
}

impl State {
    /// Can we probe drive temperature?
    pub(crate) fn can_probe_temp(&self, supports_probing_when_asleep: bool) -> bool {
        match self {
            Self::PmSuspended => false,
            Self::Standby | Self::Sleeping => supports_probing_when_asleep,
            Self::ActiveIdle | Self::Unknown => true,
        }
    }
}

/// Block device drive
pub(crate) struct Drive {
    /// Normalized (under /dev) device filepath
    pub dev_path: PathBuf,
    /// Pretty name for display
    name: String,
    /// How to probe for state
    state_probing_method: StateProbingMethod,
}

impl fmt::Display for Drive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.name)
    }
}

impl Drive {
    /// Build a drive from its device path
    pub(crate) fn new(path: &Path) -> anyhow::Result<Self> {
        let dev_path = path.canonicalize()?;
        anyhow::ensure!(
            dev_path.metadata()?.file_type().is_block_device(),
            "Path {dev_path:?} is not a block device",
        );
        let name = format!(
            "{} {}",
            dev_path
                .file_name()
                .and_then(|p| p.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid drive path"))?,
            Self::model(&dev_path)?,
        );
        let state_probing = if Self::state_hdparm(&dev_path).is_ok() {
            StateProbingMethod::Hdparm
        } else if Self::state_sdparm(&dev_path).is_ok() {
            StateProbingMethod::Sdparm
        } else {
            anyhow::bail!("Unable to probe for drive state");
        };
        log::debug!("{name}: Will use {state_probing} state probing method");
        Ok(Self {
            dev_path,
            name,
            state_probing_method: state_probing,
        })
    }

    /// Get drive model name
    fn model(path: &Path) -> anyhow::Result<String> {
        let dev = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?;
        let cmds = [["hdparm", "-I", dev], ["smartctl", "-i", dev]];
        for cmd in cmds {
            log::trace!("{}", cmd.join(" "));
            let output = Command::new(cmd[0])
                .args(&cmd[1..])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .env("LANG", "C")
                .output()?;
            if !output.status.success() {
                log::trace!("{}", output.status);
                continue;
            }
            // log::trace!("{}", std::str::from_utf8(&output.stdout).unwrap());
            if let Some(line) = output.stdout.lines().map_while(Result::ok).find_map(|l| {
                let l = l.trim_start();
                l.strip_prefix("Model Number:")
                    .or_else(|| l.strip_prefix("Product:"))
                    .map(ToOwned::to_owned)
            }) {
                return Ok(line.trim().to_owned());
            }
        }
        anyhow::bail!("Unable to get drive {path:?} model name");
    }

    /// Get drive runtime state using `hdparm`
    fn state_hdparm(path: &Path) -> anyhow::Result<State> {
        let output = Command::new("hdparm")
            .args([
                "-C",
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stdin(Stdio::null())
            .env("LANG", "C")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "hdparm failed with code {}",
            output.status
        );
        let lines: Vec<_> = output
            .stdout
            .lines()
            .chain(output.stderr.lines())
            .collect::<Result<_, _>>()?;
        anyhow::ensure!(
            !lines
                .iter()
                .any(|l| l.starts_with("SG_IO: ") && l.contains("sense data")),
            "hdparm returned soft error",
        );
        let state = lines
            .iter()
            .find_map(|l| l.trim_start().strip_prefix("drive state is: "))
            .and_then(|l| {
                l.split_ascii_whitespace()
                    .next_back()
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse hdparm drive state output"))?
            .parse()
            .unwrap_or(State::Unknown);
        Ok(state)
    }

    /// Get drive runtime state using `sdparm`
    fn state_sdparm(path: &Path) -> anyhow::Result<State> {
        let output = Command::new("sdparm")
            .args([
                "--command=ready",
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?,
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .env("LANG", "C")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "sdparm failed with code {}",
            output.status
        );
        let state = output
            .stdout
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| {
                let nl = l.trim();
                (!nl.is_empty()).then(|| nl.to_owned())
            })
            .last()
            .map(|l| match l.as_str() {
                "Ready" => State::ActiveIdle,
                "Not ready" => State::Sleeping,
                _ => State::Unknown,
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse sdparm drive state output"))?;
        Ok(state)
    }

    /// Get drive runtime state
    pub(crate) fn state(&self) -> anyhow::Result<State> {
        const SUSPENDED_PM_STATUS: [&str; 2] = ["suspended", "suspending"];
        let pm_status_path: PathBuf = [
            OsStr::new("/sys/class/block"),
            #[expect(clippy::unwrap_used)]
            self.dev_path.file_name().unwrap(),
            OsStr::new("device/power/runtime_status"),
        ]
        .into_iter()
        .collect();
        if fs::read_to_string(pm_status_path)
            .is_ok_and(|s| SUSPENDED_PM_STATUS.contains(&s.trim_end()))
        {
            Ok(State::PmSuspended)
        } else {
            match self.state_probing_method {
                StateProbingMethod::Hdparm => Self::state_hdparm(&self.dev_path),
                StateProbingMethod::Sdparm => Self::state_sdparm(&self.dev_path),
            }
        }
    }
}

#[cfg(test)]
#[expect(clippy::shadow_unrelated)]
mod tests {
    use super::*;
    use crate::tests::BinaryMock;

    #[serial_test::serial]
    #[test]
    fn test_model_hdd() {
        let _ = simple_logger::init_with_level(log::Level::Trace);

        let _hdparm_mock = BinaryMock::new("hdparm", "\n/dev/_sdX:\n\nATA device, with non-removable media\n\tModel Number:       WDC WD4003FZEX-00Z4SA0                  \n\tSerial Number:      WD-WMC5D0D4YY1K\n\tFirmware Revision:  01.01A01\n\tTransport:          Serial, SATA 1.0a, SATA II Extensions, SATA Rev 2.5, SATA Rev 2.6, SATA Rev 3.0\nStandards:\n\tSupported: 9 8 7 6 5 \n\tLikely used: 9\nConfiguration:\n\tLogical\t\tmax\tcurrent\n\tcylinders\t16383\t16383\n\theads\t\t16\t16\n\tsectors/track\t63\t63\n\t--\n\tCHS current addressable sectors:   16514064\n\tLBA    user addressable sectors:  268435455\n\tLBA48  user addressable sectors: 7814037168\n\tLogical  Sector size:                   512 bytes\n\tPhysical Sector size:                  4096 bytes\n\tLogical Sector-0 offset:                  0 bytes\n\tdevice size with M = 1024*1024:     3815447 MBytes\n\tdevice size with M = 1000*1000:     4000787 MBytes (4000 GB)\n\tcache/buffer size  = unknown\n\tNominal Media Rotation Rate: 7200\nCapabilities:\n\tLBA, IORDY(can be disabled)\n\tQueue depth: 32\n\tStandby timer values: spec'd by Standard, with device specific minimum\n\tR/W multiple sector transfer: Max = 16\tCurrent = 0\n\tDMA: mdma0 mdma1 mdma2 udma0 udma1 udma2 udma3 udma4 udma5 *udma6 \n\t     Cycle time: min=120ns recommended=120ns\n\tPIO: pio0 pio1 pio2 pio3 pio4 \n\t     Cycle time: no flow control=120ns  IORDY flow control=120ns\nCommands/features:\n\tEnabled\tSupported:\n\t   *\tSMART feature set\n\t    \tSecurity Mode feature set\n\t   *\tPower Management feature set\n\t   *\tWrite cache\n\t   *\tLook-ahead\n\t   *\tHost Protected Area feature set\n\t   *\tWRITE_BUFFER command\n\t   *\tREAD_BUFFER command\n\t   *\tNOP cmd\n\t   *\tDOWNLOAD_MICROCODE\n\t    \tPower-Up In Standby feature set\n\t   *\tSET_FEATURES required to spinup after power up\n\t    \tSET_MAX security extension\n\t   *\t48-bit Address feature set\n\t   *\tMandatory FLUSH_CACHE\n\t   *\tFLUSH_CACHE_EXT\n\t   *\tSMART error logging\n\t   *\tSMART self-test\n\t   *\tGeneral Purpose Logging feature set\n\t   *\t64-bit World wide name\n\t   *\t{READ,WRITE}_DMA_EXT_GPL commands\n\t   *\tSegmented DOWNLOAD_MICROCODE\n\t   *\tGen1 signaling speed (1.5Gb/s)\n\t   *\tGen2 signaling speed (3.0Gb/s)\n\t   *\tGen3 signaling speed (6.0Gb/s)\n\t   *\tNative Command Queueing (NCQ)\n\t   *\tHost-initiated interface power management\n\t   *\tPhy event counters\n\t   *\tNCQ priority information\n\t   *\tREAD_LOG_DMA_EXT equivalent to READ_LOG_EXT\n\t   *\tDMA Setup Auto-Activate optimization\n\t   *\tSoftware settings preservation\n\t   *\tSMART Command Transport (SCT) feature set\n\t   *\tSCT Write Same (AC2)\n\t   *\tSCT Features Control (AC4)\n\t   *\tSCT Data Tables (AC5)\n\t    \tunknown 206[12] (vendor specific)\n\t    \tunknown 206[13] (vendor specific)\n\t    \tunknown 206[14] (vendor specific)\nSecurity: \n\tMaster password revision code = 65534\n\t\tsupported\n\tnot\tenabled\n\tnot\tlocked\n\tnot\tfrozen\n\tnot\texpired: security count\n\t\tsupported: enhanced erase\n\t424min for SECURITY ERASE UNIT. 424min for ENHANCED SECURITY ERASE UNIT. \nLogical Unit WWN Device Identifier: 50014ee0593d4632\n\tNAA\t\t: 5\n\tIEEE OUI\t: 0014ee\n\tUnique ID\t: 0593d4632\nChecksum: correct\n".as_bytes(), &[], 0).unwrap();
        let _smartctl_mock = BinaryMock::new("smartctl", &[], &[], 1).unwrap();
        assert_eq!(
            Drive::model(Path::new("/dev/_sdX")).unwrap().as_str(),
            "WDC WD4003FZEX-00Z4SA0"
        );
    }

    #[serial_test::serial]
    #[test]
    fn test_model_ssd() {
        let _ = simple_logger::init_with_level(log::Level::Trace);

        let _hdparm_mock = BinaryMock::new("hdparm", "\n/dev/_sdX:".as_bytes(), &[], 0).unwrap();
        let _smartctl_mock = BinaryMock::new("smartctl", "smartctl 7.3 2022-02-28 r5338 [x86_64-linux-6.1.53-1-lts] (local build)\nCopyright (C) 2002-22, Bruce Allen, Christian Franke, www.smartmontools.org\n\n=== START OF INFORMATION SECTION ===\nModel Number:                       WD_BLACK SN850 2TB\nFirmware Version:\n                   611100WD\nPCI Vendor/Subsystem ID:            0x15b7\nIEEE OUI Identifier:                0x001b44\nTotal NVM Capacity:                 2 000 398 934 016 [2,00 TB]\nUnallocated NVM Capacity:           0\nController ID:                      8224\nNVMe Version:                       1.4\nNumber of Namespaces:               1\nNamespace 1 Size/Capacity:          2 000 398 934 016 [2,00 TB]\nNamespace 1 Formatted LBA Size:     512\nNamespace 1 IEEE EUI-64:            001b44 8b492d482c\n\n".as_bytes(), &[], 0).unwrap();
        assert_eq!(
            Drive::model(Path::new("/dev/_sdX")).unwrap().as_str(),
            "WD_BLACK SN850 2TB"
        );
    }

    #[serial_test::serial]
    #[test]
    fn test_state_hdparm() {
        let _ = simple_logger::init_with_level(log::Level::Trace);

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  active/idle\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_hdparm(Path::new("/dev/_sdX")).unwrap(),
            State::ActiveIdle
        ));

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  standby\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_hdparm(Path::new("/dev/_sdX")).unwrap(),
            State::Standby
        ));

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  sleeping\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_hdparm(Path::new("/dev/_sdX")).unwrap(),
            State::Sleeping
        ));

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  NVcache_spindown\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_hdparm(Path::new("/dev/_sdX")).unwrap(),
            State::Unknown
        ));

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  unknown\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_hdparm(Path::new("/dev/_sdX")).unwrap(),
            State::Unknown
        ));

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX: No such file or directory\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(Drive::state_hdparm(Path::new("/dev/_sdX")).is_err());

        let _hdparm_mock = BinaryMock::new(
            "hdparm",
            "\n/dev/_sdX:\n drive state is:  standby\n".as_bytes(),
            "SG_IO: bad/missing sense data, sb[]:  70 00 05 00 00 00 00 0a 00 00 00 00 20 00 01 cf 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00\n".as_bytes(),
            0,
        );
        assert!(Drive::state_hdparm(Path::new("/dev/_sdX")).is_err());
    }

    #[serial_test::serial]
    #[test]
    fn test_state_sdparm() {
        let _ = simple_logger::init_with_level(log::Level::Trace);

        let _sdparm_mock = BinaryMock::new(
            "sdparm",
            "    /dev/_sdX: SEAGATE   ST2000NM0001      0002\nReady\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_sdparm(Path::new("/dev/_sdX")).unwrap(),
            State::ActiveIdle
        ));

        let _sdparm_mock = BinaryMock::new(
            "sdparm",
            "    /dev/_sdX: SEAGATE   ST2000NM0001      0002\nNot ready\n".as_bytes(),
            &[],
            0,
        )
        .unwrap();
        assert!(matches!(
            Drive::state_sdparm(Path::new("/dev/_sdX")).unwrap(),
            State::Sleeping
        ));
    }
}
