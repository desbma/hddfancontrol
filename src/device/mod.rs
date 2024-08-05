//! Device that generates heat and its sensor

mod drive;
mod hwmon;

pub(crate) use drive::Drive;
pub(crate) use hwmon::Hwmon;
