//! Temperature logging to JSONL file

use std::{
    fmt,
    fs::File,
    io::{BufWriter, Write as _},
    path::Path,
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::probe::Temp;

/// A single device temperature measurement
#[derive(Debug, Serialize)]
pub(crate) struct TempMeasure {
    /// Device display name
    device: String,
    /// Temperature in Celsius, or `None` if the drive is sleeping
    temp_celcius: Option<Temp>,
}

impl TempMeasure {
    /// Create a new temperature measurement
    pub(crate) fn new<D>(device: D, temp_celcius: Option<Temp>) -> Self
    where
        D: fmt::Display,
    {
        Self {
            device: device.to_string(),
            temp_celcius,
        }
    }
}

/// A timestamped set of temperature measurements for all devices
#[derive(Debug, Serialize)]
struct TempLog {
    /// UTC timestamp
    time_utc: DateTime<Utc>,
    /// Per-device measurements
    measures: Vec<TempMeasure>,
}

/// Writer that appends temperature log entries to a JSONL file
pub(crate) struct TempLogWriter {
    /// Output file handle
    writer: BufWriter<File>,
}

impl TempLogWriter {
    /// Open (or create) a JSONL file for appending
    pub(crate) fn new(path: &Path) -> anyhow::Result<Self> {
        let file = File::options().create(true).append(true).open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Write a single log entry as a JSONL line
    pub(crate) fn write(&mut self, measures: Vec<TempMeasure>) -> anyhow::Result<()> {
        let entry = TempLog {
            time_utc: Utc::now(),
            measures,
        };
        serde_json::to_writer(&mut self.writer, &entry)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn json_line_format() {
        let entry = TempLog {
            time_utc: "2026-02-13T12:50:01Z".parse().unwrap(),
            measures: vec![
                TempMeasure::new("Seagate model blah XYZ".to_owned(), Some(48.123)),
                TempMeasure::new("WD model ABC".to_owned(), None),
            ],
        };
        let line = serde_json::to_string(&entry).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["time_utc"], "2026-02-13T12:50:01Z");
        assert_eq!(v["measures"][0]["device"], "Seagate model blah XYZ");
        assert_eq!(v["measures"][0]["temp_celcius"], 48.123);
        assert_eq!(v["measures"][1]["device"], "WD model ABC");
        assert!(v["measures"][1]["temp_celcius"].is_null());
    }

    #[test]
    fn json_line_is_single_line() {
        let entry = TempLog {
            time_utc: "2026-02-13T12:50:01Z".parse().unwrap(),
            measures: vec![TempMeasure::new("drive".to_owned(), Some(42.0))],
        };
        let line = serde_json::to_string(&entry).unwrap();
        assert!(!line.contains('\n'));
    }

    #[test]
    fn json_line_all_sleeping() {
        let entry = TempLog {
            time_utc: "2026-02-13T00:00:00Z".parse().unwrap(),
            measures: vec![
                TempMeasure::new("drive1".to_owned(), None),
                TempMeasure::new("drive2".to_owned(), None),
            ],
        };
        let line = serde_json::to_string(&entry).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert!(v["measures"][0]["temp_celcius"].is_null());
        assert!(v["measures"][1]["temp_celcius"].is_null());
    }

    #[test]
    fn writer_appends_lines() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = TempLogWriter::new(file.path()).unwrap();

        writer
            .write(vec![TempMeasure::new("d1".to_owned(), Some(40.0))])
            .unwrap();
        writer
            .write(vec![TempMeasure::new("d1".to_owned(), Some(41.0))])
            .unwrap();

        let contents = fs::read_to_string(file.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        let v1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        let v2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(v1["measures"][0]["temp_celcius"], 40.0);
        assert_eq!(v2["measures"][0]["temp_celcius"], 41.0);
    }
}
