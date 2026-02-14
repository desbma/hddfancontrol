//! Temperature logging to JSONL file with daily rotation and gzip compression

use std::{
    fmt,
    fs::{self, File},
    io::{self, BufRead as _, BufReader, BufWriter, Write as _},
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use chrono::{DateTime, NaiveDate, Utc};
use flate2::{Compression, bufread, write::GzEncoder};
use itertools::Itertools as _;
use rev_lines::RawRevLines;

use crate::probe::Temp;

/// A single device temperature measurement
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct TempLog {
    /// UTC timestamp
    time_utc: DateTime<Utc>,
    /// Per-device measurements
    measures: Vec<TempMeasure>,
}

/// Logger that appends temperature entries to a JSONL file with daily rotation
pub(crate) struct TempLogger {
    /// Output file path
    path: PathBuf,
    /// File name stem
    stem: String,
    /// Output file handle
    writer: BufWriter<File>,
    /// Date of the last write (UTC), used to trigger daily rotation
    last_date: Option<NaiveDate>,
    /// Maximum number of rotated files to keep
    max_files: Option<NonZeroUsize>,
}

/// Extension of a log file after rotation & compression
const ROTATED_FILE_SUFFIX: &str = ".jsonl.gz";

impl TempLogger {
    /// Open (or create) a JSONL file for appending
    pub(crate) fn new(path: &Path, max_files: Option<NonZeroUsize>) -> anyhow::Result<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid log file path"))?
            .to_owned();
        let file = File::options().create(true).append(true).open(path)?;
        let last_date = Self::last_logged_date(path, &stem)
            .inspect_err(|err| log::warn!("Failed to read last logged date: {err:?}"))
            .ok();
        Ok(Self {
            path: path.to_owned(),
            stem,
            writer: BufWriter::new(file),
            last_date,
            max_files,
        })
    }

    /// Read the date of the last logged entry from the JSONL file, or from
    /// the most recent rotated gzip archive if the JSONL file is empty
    fn last_logged_date(path: &Path, stem: &str) -> anyhow::Result<NaiveDate> {
        let file = File::open(path)?;
        let last_entry: TempLog = if let Some(line) = RawRevLines::new(file).next().transpose()? {
            serde_json::from_slice(&line)?
        } else {
            // JSONL file is empty, try the most recent rotated archive
            let gz_path = path.with_file_name(format!("{stem}_00000001{ROTATED_FILE_SUFFIX}"));
            let line = BufReader::new(bufread::GzDecoder::new(BufReader::new(File::open(
                &gz_path,
            )?)))
            .lines()
            .take_while_inclusive(Result::is_ok)
            .last()
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Rotated file {gz_path:?} is empty"))?;
            serde_json::from_str(&line)?
        };
        Ok(last_entry.time_utc.date_naive())
    }

    /// Log a set of temperature measurements, rotating daily
    pub(crate) fn log(
        &mut self,
        time: DateTime<Utc>,
        measures: Vec<TempMeasure>,
    ) -> anyhow::Result<()> {
        let today = time.date_naive();

        // Rotate if the day changed
        if self.last_date.is_some_and(|prev| today > prev) {
            self.writer.flush()?;
            self.rotate()?;
            self.writer =
                BufWriter::new(File::options().create(true).append(true).open(&self.path)?);
        }
        self.last_date = self.last_date.max(Some(today));

        self.write_jsonl(time, measures)
    }

    /// Serialize and write a single JSONL line
    fn write_jsonl(
        &mut self,
        time_utc: DateTime<Utc>,
        measures: Vec<TempMeasure>,
    ) -> anyhow::Result<()> {
        let entry = TempLog { time_utc, measures };
        serde_json::to_writer(&mut self.writer, &entry)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Find the highest existing rotated file index (1-based), or 0 if none exist
    fn max_rotation_index(&self) -> anyhow::Result<u32> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let prefix = format!("{}_", self.stem);
        Ok(fs::read_dir(dir)?
            .filter_map(Result::ok)
            .filter_map(|e| {
                e.file_name()
                    .to_str()?
                    .strip_prefix(&prefix)?
                    .strip_suffix(ROTATED_FILE_SUFFIX)?
                    .parse::<u32>()
                    .ok()
            })
            .max()
            .unwrap_or(0))
    }

    /// Build the rotated file path for a given index
    fn rotated_path(&self, index: u32) -> PathBuf {
        self.path
            .with_file_name(format!("{}_{index:08}{ROTATED_FILE_SUFFIX}", self.stem))
    }

    /// Rotate the current log file: shift existing rotated files up by one,
    /// compress the current file into the first slot, and remove it.
    ///
    /// When `max_files` is set, the shift is capped so the oldest file
    /// beyond the limit is silently overwritten by `fs::rename`
    fn rotate(&mut self) -> anyhow::Result<()> {
        let max_idx = self.max_rotation_index()?;

        // Shift existing rotated files up by one, starting from highest to avoid overwrites.
        // When max_files is set, skip the highest index so it gets overwritten,
        // effectively discarding the oldest file.
        #[expect(clippy::cast_possible_truncation)]
        let start = match self.max_files {
            Some(max) => max_idx.min(max.get() as u32 - 1),
            None => max_idx,
        };
        for idx in (1..=start).rev() {
            let src = self.rotated_path(idx);
            let dst = self.rotated_path(idx + 1);
            log::trace!("{src:?} -> {dst:?}");
            fs::rename(src, dst)?;
        }

        // Compress current file
        let dst = self.rotated_path(1);
        debug_assert!(!dst.is_file());
        self.writer.flush()?;
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut file_writer = BufWriter::new(File::create(&dst)?);
        let mut gz_writer = GzEncoder::new(&mut file_writer, Compression::best());
        io::copy(&mut reader, &mut gz_writer)?;
        gz_writer.finish()?;
        file_writer.flush()?;

        // Remove and reopen
        fs::remove_file(&self.path)?;
        self.writer = BufWriter::new(File::options().create(true).append(true).open(&self.path)?);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use flate2::read;

    use super::*;

    /// Decompress a gzip file and return its contents as a string
    fn decompress_gz(path: &Path) -> String {
        let file = File::open(path).unwrap();
        let mut decoder = read::GzDecoder::new(file);
        let mut content = String::new();
        decoder.read_to_string(&mut content).unwrap();
        content
    }

    /// Parse a UTC datetime string
    fn utc(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn json_line_format() {
        let entry = TempLog {
            time_utc: utc("2026-02-13T12:50:01Z"),
            measures: vec![
                TempMeasure::new("Seagate model blah XYZ".to_owned(), Some(48.123)),
                TempMeasure::new("WD model ABC".to_owned(), None),
            ],
        };
        let line = serde_json::to_string(&entry).unwrap();
        assert_eq!(
            line,
            r#"{"time_utc":"2026-02-13T12:50:01Z","measures":[{"device":"Seagate model blah XYZ","temp_celcius":48.123},{"device":"WD model ABC","temp_celcius":null}]}"#
        );
    }

    #[test]
    fn logger_appends_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let mut logger = TempLogger::new(&log_path, None).unwrap();

        logger
            .log(
                utc("2026-02-13T10:00:00Z"),
                vec![
                    TempMeasure::new("drive1", Some(40.0)),
                    TempMeasure::new("drive2", None),
                ],
            )
            .unwrap();
        logger
            .log(
                utc("2026-02-13T10:00:30Z"),
                vec![
                    TempMeasure::new("drive1", Some(41.0)),
                    TempMeasure::new("drive2", Some(38.0)),
                ],
            )
            .unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            r#"{"time_utc":"2026-02-13T10:00:00Z","measures":[{"device":"drive1","temp_celcius":40.0},{"device":"drive2","temp_celcius":null}]}"#
        );
        assert_eq!(
            lines[1],
            r#"{"time_utc":"2026-02-13T10:00:30Z","measures":[{"device":"drive1","temp_celcius":41.0},{"device":"drive2","temp_celcius":38.0}]}"#
        );
    }

    #[test]
    fn log_rotates_on_day_change() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz1 = dir.path().join("temps_00000001.jsonl.gz");
        let mut logger = TempLogger::new(&log_path, None).unwrap();

        // Write two entries on day 1
        logger
            .log(
                utc("2026-02-13T23:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();
        logger
            .log(
                utc("2026-02-13T23:30:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        // Day changes: should trigger rotation
        logger
            .log(
                utc("2026-02-14T00:05:00Z"),
                vec![TempMeasure::new("d1", Some(42.0))],
            )
            .unwrap();

        // Rotated file should contain day 1 entries
        assert!(gz1.exists());
        let rotated = decompress_gz(&gz1);
        let rotated_lines: Vec<&str> = rotated.lines().collect();
        assert_eq!(rotated_lines.len(), 2);
        assert_eq!(
            rotated_lines[0],
            r#"{"time_utc":"2026-02-13T23:00:00Z","measures":[{"device":"d1","temp_celcius":40.0}]}"#
        );
        assert_eq!(
            rotated_lines[1],
            r#"{"time_utc":"2026-02-13T23:30:00Z","measures":[{"device":"d1","temp_celcius":41.0}]}"#
        );

        // Current file should contain only the day 2 entry
        let current = fs::read_to_string(&log_path).unwrap();
        let current_lines: Vec<&str> = current.lines().collect();
        assert_eq!(current_lines.len(), 1);
        assert_eq!(
            current_lines[0],
            r#"{"time_utc":"2026-02-14T00:05:00Z","measures":[{"device":"d1","temp_celcius":42.0}]}"#
        );
    }

    #[test]
    fn max_rotation_index_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let logger = TempLogger::new(&log_path, None).unwrap();
        assert_eq!(logger.max_rotation_index().unwrap(), 0);
    }

    #[test]
    fn max_rotation_index_with_existing() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        File::create(dir.path().join("temps_00000001.jsonl.gz")).unwrap();
        File::create(dir.path().join("temps_00000003.jsonl.gz")).unwrap();
        let logger = TempLogger::new(&log_path, None).unwrap();
        assert_eq!(logger.max_rotation_index().unwrap(), 3);
    }

    #[test]
    fn rotated_path_with_extension() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let logger = TempLogger::new(&log_path, None).unwrap();
        assert_eq!(
            logger.rotated_path(1),
            dir.path().join("temps_00000001.jsonl.gz")
        );
        assert_eq!(
            logger.rotated_path(42),
            dir.path().join("temps_00000042.jsonl.gz")
        );
    }

    #[test]
    fn rotated_path_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps");
        let logger = TempLogger::new(&log_path, None).unwrap();
        assert_eq!(
            logger.rotated_path(1),
            dir.path().join("temps_00000001.jsonl.gz")
        );
        assert_eq!(
            logger.rotated_path(42),
            dir.path().join("temps_00000042.jsonl.gz")
        );
    }

    #[test]
    fn rotate_with_extension() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        fs::write(&log_path, "data\n").unwrap();

        let mut logger = TempLogger::new(&log_path, None).unwrap();
        logger.rotate().unwrap();

        let gz_path = dir.path().join("temps_00000001.jsonl.gz");
        assert!(gz_path.exists());
        assert!(fs::read(&log_path).unwrap().is_empty());
        assert_eq!(decompress_gz(&gz_path), "data\n");
    }

    #[test]
    fn rotate_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps");
        fs::write(&log_path, "data\n").unwrap();

        let mut logger = TempLogger::new(&log_path, None).unwrap();
        logger.rotate().unwrap();

        let gz_path = dir.path().join("temps_00000001.jsonl.gz");
        assert!(gz_path.exists());
        assert!(fs::read(&log_path).unwrap().is_empty());
        assert_eq!(decompress_gz(&gz_path), "data\n");
    }

    #[test]
    fn rotate_shifts_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz = |i| dir.path().join(format!("temps_{i:08}.jsonl.gz"));
        let mut logger = TempLogger::new(&log_path, None).unwrap();

        // First rotation: temps.jsonl -> temps_00000001.jsonl.gz
        fs::write(&log_path, "day1\n").unwrap();
        logger.rotate().unwrap();
        assert!(gz(1).exists());
        assert!(!gz(2).exists());
        assert_eq!(decompress_gz(&gz(1)), "day1\n");

        // Second rotation: existing _00000001 shifts to _00000002,
        // new temps.jsonl becomes _00000001
        fs::write(&log_path, "day2\n").unwrap();
        logger.rotate().unwrap();
        assert!(gz(1).exists());
        assert!(gz(2).exists());
        assert!(!gz(3).exists());
        // _00000001 should be the newest (day2), _00000002 the oldest (day1)
        assert_eq!(decompress_gz(&gz(1)), "day2\n");
        assert_eq!(decompress_gz(&gz(2)), "day1\n");

        // Third rotation: _00000002 -> _00000003, _00000001 -> _00000002,
        // temps.jsonl -> _00000001
        fs::write(&log_path, "day3\n").unwrap();
        logger.rotate().unwrap();
        assert!(gz(1).exists());
        assert!(gz(2).exists());
        assert!(gz(3).exists());
        assert_eq!(decompress_gz(&gz(1)), "day3\n");
        assert_eq!(decompress_gz(&gz(2)), "day2\n");
        assert_eq!(decompress_gz(&gz(3)), "day1\n");
    }

    #[test]
    fn rotate_discards_oldest_when_at_limit() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz = |i| dir.path().join(format!("temps_{i:08}.jsonl.gz"));
        let mut logger = TempLogger::new(&log_path, NonZeroUsize::new(3)).unwrap();

        // Build up 3 rotated files
        for day in 1..=3 {
            fs::write(&log_path, format!("day{day}\n")).unwrap();
            logger.rotate().unwrap();
        }
        assert_eq!(decompress_gz(&gz(1)), "day3\n");
        assert_eq!(decompress_gz(&gz(2)), "day2\n");
        assert_eq!(decompress_gz(&gz(3)), "day1\n");
        assert!(!gz(4).exists());

        // 4th rotation: _00000003 (day1) should be overwritten by _00000002 (day2),
        // only 3 rotated files remain
        fs::write(&log_path, "day4\n").unwrap();
        logger.rotate().unwrap();
        assert_eq!(decompress_gz(&gz(1)), "day4\n");
        assert_eq!(decompress_gz(&gz(2)), "day3\n");
        assert_eq!(decompress_gz(&gz(3)), "day2\n");
        assert!(!gz(4).exists());
    }

    #[test]
    fn time_jump_backward_no_spurious_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz1 = dir.path().join("temps_00000001.jsonl.gz");
        let mut logger = TempLogger::new(&log_path, None).unwrap();

        // Write on Feb 15
        logger
            .log(
                utc("2026-02-15T12:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();

        // Clock jumps back to Feb 14
        logger
            .log(
                utc("2026-02-14T12:00:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        // No rotation should have happened
        assert!(!gz1.exists());

        // Clock catches up to Feb 15 again: still no rotation,
        // because we never truly crossed a new day boundary
        logger
            .log(
                utc("2026-02-15T18:00:00Z"),
                vec![TempMeasure::new("d1", Some(42.0))],
            )
            .unwrap();
        assert!(!gz1.exists());

        // All 3 entries should be in the current file
        let contents = fs::read_to_string(&log_path).unwrap();
        assert_eq!(contents.lines().count(), 3);
    }

    #[test]
    fn time_jump_backward_then_forward_rotates_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz1 = dir.path().join("temps_00000001.jsonl.gz");
        let mut logger = TempLogger::new(&log_path, None).unwrap();

        // Write on Feb 15
        logger
            .log(
                utc("2026-02-15T12:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();

        // Clock jumps back to Feb 14
        logger
            .log(
                utc("2026-02-14T12:00:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        // Clock advances to Feb 16: this is a genuine new day, should rotate
        logger
            .log(
                utc("2026-02-16T06:00:00Z"),
                vec![TempMeasure::new("d1", Some(42.0))],
            )
            .unwrap();

        assert!(gz1.exists());
        let rotated = decompress_gz(&gz1);
        // Rotated file should contain the 2 entries written before the rotation
        assert_eq!(rotated.lines().count(), 2);
        // Current file should contain only the Feb 16 entry
        let current = fs::read_to_string(&log_path).unwrap();
        assert_eq!(current.lines().count(), 1);
    }

    #[test]
    fn rotate_noop_cleanup_when_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz = |i| dir.path().join(format!("temps_{i:08}.jsonl.gz"));
        let mut logger = TempLogger::new(&log_path, NonZeroUsize::new(5)).unwrap();

        // Only 2 rotations, well under the limit of 5
        fs::write(&log_path, "day1\n").unwrap();
        logger.rotate().unwrap();
        fs::write(&log_path, "day2\n").unwrap();
        logger.rotate().unwrap();

        assert!(gz(1).exists());
        assert!(gz(2).exists());
        assert_eq!(decompress_gz(&gz(1)), "day2\n");
        assert_eq!(decompress_gz(&gz(2)), "day1\n");
    }

    #[test]
    fn last_logged_date_from_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let line = r#"{"time_utc":"2026-02-13T23:30:00Z","measures":[{"device":"d1","temp_celcius":40.0}]}"#;
        fs::write(&log_path, format!("{line}\n")).unwrap();

        let date = TempLogger::last_logged_date(&log_path, "temps").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 13).unwrap());
    }

    #[test]
    fn last_logged_date_from_jsonl_multiple_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let line1 = r#"{"time_utc":"2026-02-13T10:00:00Z","measures":[{"device":"d1","temp_celcius":40.0}]}"#;
        let line2 = r#"{"time_utc":"2026-02-14T23:59:00Z","measures":[{"device":"d1","temp_celcius":41.0}]}"#;
        fs::write(&log_path, format!("{line1}\n{line2}\n")).unwrap();

        let date = TempLogger::last_logged_date(&log_path, "temps").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
    }

    #[test]
    fn last_logged_date_from_gz_when_jsonl_empty() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        File::create(&log_path).unwrap();

        // Create a compressed rotated file
        let gz_path = dir.path().join("temps_00000001.jsonl.gz");
        let line = r#"{"time_utc":"2026-02-12T20:00:00Z","measures":[{"device":"d1","temp_celcius":39.0}]}"#;
        let mut gz = GzEncoder::new(
            BufWriter::new(File::create(&gz_path).unwrap()),
            Compression::fast(),
        );
        gz.write_all(format!("{line}\n").as_bytes()).unwrap();
        gz.finish().unwrap();

        let date = TempLogger::last_logged_date(&log_path, "temps").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 12).unwrap());
    }

    #[test]
    fn last_logged_date_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        assert!(TempLogger::last_logged_date(&log_path, "temps").is_err());
    }

    #[test]
    fn last_logged_date_empty_no_gz() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        File::create(&log_path).unwrap();
        assert!(TempLogger::last_logged_date(&log_path, "temps").is_err());
    }

    #[test]
    fn constructor_seeds_last_date_from_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz1 = dir.path().join("temps_00000001.jsonl.gz");

        // Write an entry on Feb 13
        let mut logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-13T23:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();

        assert!(!gz1.exists());

        // Simulate daemon restart on Feb 14: constructor should seed last_date
        // from the existing file, so writing on Feb 14 triggers rotation
        logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-14T00:01:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        assert!(gz1.exists());
        let rotated = decompress_gz(&gz1);
        assert_eq!(rotated.lines().count(), 1);
        let current = fs::read_to_string(&log_path).unwrap();
        assert_eq!(current.lines().count(), 1);
    }

    #[test]
    fn constructor_seeds_last_date_from_gz_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");

        // Write and rotate to create a gz archive, then clear the jsonl file
        let mut logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-13T23:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();

        // Simulate: the jsonl was rotated on a previous run, now it's empty
        // and the gz has the last entry
        let content = fs::read_to_string(&log_path).unwrap();
        let gz_path = dir.path().join("temps_00000001.jsonl.gz");
        let mut gz = GzEncoder::new(
            BufWriter::new(File::create(&gz_path).unwrap()),
            Compression::fast(),
        );
        gz.write_all(content.as_bytes()).unwrap();
        gz.finish().unwrap();
        fs::write(&log_path, "").unwrap();

        // Restart on Feb 14: should seed from gz and rotate
        logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-14T00:01:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        // The gz_00000001 was shifted to gz_00000002, and the empty file
        // was compressed to gz_00000001 (empty rotation)
        let gz2_path = dir.path().join("temps_00000002.jsonl.gz");
        assert!(gz2_path.exists());
        let current = fs::read_to_string(&log_path).unwrap();
        assert_eq!(current.lines().count(), 1);
    }

    #[test]
    fn constructor_no_rotation_same_day() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("temps.jsonl");
        let gz1 = dir.path().join("temps_00000001.jsonl.gz");

        // Write an entry on Feb 13
        let mut logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-13T10:00:00Z"),
                vec![TempMeasure::new("d1", Some(40.0))],
            )
            .unwrap();

        // Restart on the same day: no rotation should happen
        logger = TempLogger::new(&log_path, None).unwrap();
        logger
            .log(
                utc("2026-02-13T12:00:00Z"),
                vec![TempMeasure::new("d1", Some(41.0))],
            )
            .unwrap();

        assert!(!gz1.exists());
        let contents = fs::read_to_string(&log_path).unwrap();
        assert_eq!(contents.lines().count(), 2);
    }
}
