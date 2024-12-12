//! Shared code for tests

#![allow(clippy::unwrap_used)]

use std::{env, fs::OpenOptions, io::Write as _, os::unix::prelude::OpenOptionsExt as _};

/// A mocked binary added in PATH env var
pub(crate) struct BinaryMock {
    bin_dir: tempfile::TempDir,
    _stdout: tempfile::NamedTempFile,
    _stderr: tempfile::NamedTempFile,
}

const PATH_KEY: &str = "PATH";

impl BinaryMock {
    /// Create a mock binary available in PATH
    pub(crate) fn new(
        name: &str,
        stdout_data: &[u8],
        stderr_data: &[u8],
        rc: u8,
    ) -> anyhow::Result<Self> {
        // Create temporary files
        let bin_dir = tempfile::tempdir()?;
        let stdout = tempfile::NamedTempFile::new()?;
        let stderr = tempfile::NamedTempFile::new()?;

        // Write stdouyt/stderr data to files
        stdout.reopen()?.write_all(stdout_data)?;
        stderr.reopen()?.write_all(stderr_data)?;

        // Create fake binary
        let bin_filepath = bin_dir.path().join(name);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(bin_filepath)?
            .write_all(
                format!(
                    "#!/bin/sh -e\ncat {}\ncat {} >&2\nexit {}",
                    stdout.path().to_str().unwrap(),
                    stderr.path().to_str().unwrap(),
                    rc
                )
                .as_bytes(),
            )?;

        Self::add_path_dir(bin_dir.path())?;

        Ok(Self {
            bin_dir,
            _stdout: stdout,
            _stderr: stderr,
        })
    }

    /// Add directory to PATH env var
    fn add_path_dir(path: &std::path::Path) -> anyhow::Result<()> {
        let var = env::var_os(PATH_KEY)
            .ok_or_else(|| anyhow::anyhow!("{PATH_KEY} env var is not set"))?;
        log::trace!("Before: PATH={}", var.to_str().unwrap());
        let mut dirs = env::split_paths(&var).collect::<Vec<_>>();
        dirs.insert(0, path.to_owned());
        let new_var = env::join_paths(dirs)?;
        env::set_var(PATH_KEY, &new_var);
        log::trace!("After: PATH={}", new_var.to_str().unwrap());
        Ok(())
    }

    /// Remove directory from PATH env var
    fn remove_path_dir(path: &std::path::Path) -> anyhow::Result<()> {
        let var = env::var_os(PATH_KEY)
            .ok_or_else(|| anyhow::anyhow!("{PATH_KEY} env var is not set"))?;
        log::trace!("Before: PATH={}", var.to_str().unwrap());
        let dirs = env::split_paths(&var)
            .filter(|p| p != path)
            .collect::<Vec<_>>();
        let new_var = env::join_paths(dirs)?;
        env::set_var(PATH_KEY, &new_var);
        log::trace!("After: PATH={}", new_var.to_str().unwrap());
        Ok(())
    }
}

impl Drop for BinaryMock {
    fn drop(&mut self) {
        let _ = Self::remove_path_dir(self.bin_dir.path());
    }
}
