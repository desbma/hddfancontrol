//! Device that generates heat and its sensor

mod drive;
mod hwmon;

pub use drive::Drive;
pub use hwmon::Hwmon;
