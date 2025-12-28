//! Hddtemp temperature probing

use std::{
    fmt,
    io::Read as _,
    net::{SocketAddrV4, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
    str,
};

use itertools::Itertools as _;

use super::{DeviceTempProber, Drive, DriveTempProbeMethod, ProberError, Temp};

/// Hddtemp daemon probing method
pub(crate) struct DaemonMethod {
    /// Daemon address
    pub addr: SocketAddrV4,
}

impl DriveTempProbeMethod for DaemonMethod {
    type Prober = DaemonProber;

    fn prober(&self, drive: &Drive) -> Result<DaemonProber, ProberError> {
        let mut prober = DaemonProber {
            addr: self.addr,
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(prober)
    }

    fn supports_probing_sleeping(&self) -> bool {
        false
    }
}

impl fmt::Display for DaemonMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "hddtemp daemon")
    }
}

/// Hddtemp daemon temperature prober
pub(crate) struct DaemonProber {
    /// Daemon address
    addr: SocketAddrV4,
    /// Device path in /dev/
    device: PathBuf,
}

impl DeviceTempProber for DaemonProber {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let mut stream = TcpStream::connect(self.addr)?;
        let mut buf = String::new();
        stream.read_to_string(&mut buf)?;
        let mut tokens = buf.split('|');
        while let Some(chunk) = tokens.next_array::<5>() {
            let dev = chunk[1];
            // At this point we have already converted the device path to string
            #[expect(clippy::unwrap_used)]
            if dev != self.device.to_str().unwrap() {
                continue;
            }
            let mut temp = chunk[3].parse()?;
            let unit = chunk[4];
            if unit == "F" {
                temp = (temp - 32.0) / 1.8;
            } else if unit != "C" {
                anyhow::bail!("Unexpected temp unit {unit:?}");
            }
            return Ok(temp);
        }
        anyhow::bail!("No temperature found for device {:?}", self.device);
    }
}

/// Hddtemp invocation probing method
pub(crate) struct InvocationMethod;

impl DriveTempProbeMethod for InvocationMethod {
    type Prober = InvocationProber;

    fn prober(&self, drive: &Drive) -> Result<InvocationProber, ProberError> {
        let mut prober = InvocationProber {
            device: drive.dev_path.clone(),
        };
        prober
            .probe_temp()
            .map_err(|e| ProberError::Unsupported(e.to_string()))?;
        Ok(prober)
    }

    fn supports_probing_sleeping(&self) -> bool {
        false
    }
}

impl fmt::Display for InvocationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "hddtemp invocation")
    }
}

/// Hddtemp invocation temperature prober
pub(crate) struct InvocationProber {
    /// Device path in /dev/
    device: PathBuf,
}

impl DeviceTempProber for InvocationProber {
    fn probe_temp(&mut self) -> anyhow::Result<Temp> {
        let output = Command::new("hddtemp")
            .args([
                "-u",
                "C",
                "-n",
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
            "hddtemp failed with code {}",
            output.status
        );
        // TODO handle "drive is sleeping" case
        let temp = str::from_utf8(&output.stdout)?.trim_end().parse()?;
        Ok(temp)
    }
}

#[expect(clippy::shadow_unrelated)]
#[cfg(test)]
mod tests {
    use std::{
        io::{ErrorKind, Write as _},
        net::{Ipv4Addr, TcpListener},
        sync::mpsc,
        thread,
    };

    use float_cmp::approx_eq;

    use super::*;
    use crate::tests::BinaryMock;

    fn start_hddtemp_server() -> anyhow::Result<(SocketAddrV4, mpsc::Sender<Vec<u8>>)> {
        let mut port = 1024;
        let (addr, listener) = loop {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            let res = TcpListener::bind(addr);
            let listener = match res {
                Ok(l) => l,
                Err(e) if e.kind() == ErrorKind::AddrInUse => {
                    port += 1;
                    continue;
                }
                Err(_) => res?,
            };
            break (addr, listener);
        };
        let (chan_tx, chan_rx) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            while let (Ok(msg), Ok((mut sckt, _addr))) = (chan_rx.recv(), listener.accept()) {
                if sckt.write_all(&msg).is_err() {
                    break;
                }
            }
        });
        Ok((addr, chan_tx))
    }

    #[test]
    fn daemon_probe_temp() {
        let (addr, msg_tx) = start_hddtemp_server().unwrap();
        let mut prober = DaemonProber {
            addr,
            device: PathBuf::from("/dev/_sdz"),
        };

        msg_tx.send(b"|/dev/_sdz|DriveSDZ|30|C|".to_vec()).unwrap();
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));

        msg_tx
            .send(b"|/dev/_sdy|DriveSDY|31|C||/dev/_sdz|DriveSDZ|30|C|".to_vec())
            .unwrap();
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));

        msg_tx
            .send(
                b"|/dev/_sdy|DriveSDY|31|C||/dev/_sdz|DriveSDZ|30|C||/dev/_sdx|DriveSDX|32|C|"
                    .to_vec(),
            )
            .unwrap();
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));

        // TODO special error for this?
        msg_tx.send(b"|/dev/_sdz|DriveSDZ|SLP|*|".to_vec()).unwrap();
        assert!(prober.probe_temp().is_err());

        msg_tx.send(b"|/dev/_sdz|DriveSDZ|ERR|*|".to_vec()).unwrap();
        assert!(prober.probe_temp().is_err());

        msg_tx
            .send(b"|/dev/_sdx|DriveSDX|31|C||/dev/_sdy|DriveSDY|32|C|".to_vec())
            .unwrap();
        assert!(prober.probe_temp().is_err());

        msg_tx.send(b"|/dev/_sdz|DriveSDZ|86|F|".to_vec()).unwrap();
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 30.0));
    }

    #[serial_test::serial]
    #[test]
    fn invocation_probe_temp() {
        let mut prober = InvocationProber {
            device: PathBuf::from("/dev/_sdX"),
        };

        let _hddtemp = BinaryMock::new("hddtemp", "35\n".as_bytes(), &[], 0);
        assert!(approx_eq!(f64, prober.probe_temp().unwrap(), 35.0));

        let _hddtemp = BinaryMock::new(
            "hddtemp",
            "/dev/_sdX: drive_name: drive is sleeping\n".as_bytes(),
            &[],
            0,
        );
        assert!(prober.probe_temp().is_err());
    }
}
