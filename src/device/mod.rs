//! Device that generates heat and its sensor

mod drive;
mod hwmon;

pub(crate) use drive::{Drive, StateError};
pub(crate) use hwmon::Hwmon;
