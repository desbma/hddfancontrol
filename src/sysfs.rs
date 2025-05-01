//! Helpers to manipulate sysfs files

use std::{
    error::Error,
    fmt,
    fs::File,
    io::{Read as _, Write as _},
    os::linux::fs::MetadataExt as _,
    path::{Path, PathBuf},
    str::{self, FromStr},
};

use nix::sys::stat;

/// Ensure path is a valid sysfs file path, and normalizes it
pub(crate) fn ensure_sysfs_file(path: &Path) -> anyhow::Result<PathBuf> {
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
pub(crate) fn ensure_sysfs_dir(path: &Path) -> anyhow::Result<PathBuf> {
    let path = path.canonicalize()?;
    anyhow::ensure!(path.is_dir(), "{path:?} missing or not a directory");
    Ok(path)
}

/// Write integer value to path
pub(crate) fn write_value<T>(path: &Path, val: T) -> anyhow::Result<()>
where
    T: fmt::Display,
{
    let mut f = File::create(path)?;
    f.write_all(format!("{val}\n").as_bytes())?;
    Ok(())
}

/// Read integer value from path
pub(crate) fn read_value<T>(path: &Path) -> anyhow::Result<T>
where
    T: FromStr + PartialEq + Copy,
    <T as FromStr>::Err: Error + Send + Sync,
    <T as FromStr>::Err: 'static,
{
    let mut file = File::open(path)?;
    let mut buf = vec![0; 16];
    let count = file.read(&mut buf)?;
    buf.truncate(count);
    let s = str::from_utf8(&buf)?.trim_end();
    Ok(s.parse::<T>()?)
}
