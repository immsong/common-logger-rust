use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::{LogTimeZone, LoggerConfig};

pub struct DailyFileAppender {
    log_dir: PathBuf,
    filename_prefix: String,
    max_log_files: usize,
    time_zone: LogTimeZone,
    current_date: NaiveDate,
    file: File,
}

impl DailyFileAppender {
    pub fn new(config: &LoggerConfig) -> io::Result<Self> {
        fs::create_dir_all(&config.log_dir)?;

        let current_date = config.time_zone.current_date();
        let file = Self::open_log_file(&config.log_dir, &config.filename_prefix, current_date)?;

        let appender = Self {
            log_dir: config.log_dir.clone(),
            filename_prefix: config.filename_prefix.clone(),
            max_log_files: config.max_log_files,
            time_zone: config.time_zone,
            current_date,
            file,
        };

        appender.prune_old_logs();

        Ok(appender)
    }

    fn open_log_file(log_dir: &Path, filename_prefix: &str, date: NaiveDate) -> io::Result<File> {
        let filename = format!("{}.{}.log", filename_prefix, date.format("%Y-%m-%d"),);

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(filename))
    }

    fn rotate_if_needed(&mut self) -> io::Result<()> {
        let current_date = self.time_zone.current_date();

        if current_date == self.current_date {
            return Ok(());
        }

        let file = Self::open_log_file(&self.log_dir, &self.filename_prefix, current_date)?;

        self.file.flush()?;
        self.file = file;
        self.current_date = current_date;

        self.prune_old_logs();

        Ok(())
    }

    fn prune_old_logs(&self) {
        if self.max_log_files == 0 {
            return;
        }

        let Ok(entries) = fs::read_dir(&self.log_dir) else {
            return;
        };

        let prefix = format!("{}.", self.filename_prefix);

        let mut log_files = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let metadata = entry.metadata().ok()?;

                if !metadata.is_file() {
                    return None;
                }

                let filename = entry.file_name();
                let filename = filename.to_str()?;

                let date = filename.strip_prefix(&prefix)?.strip_suffix(".log")?;

                let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;

                Some((date, entry.path()))
            })
            .collect::<Vec<_>>();

        if log_files.len() <= self.max_log_files {
            return;
        }

        log_files.sort_by_key(|(date, _)| *date);

        let remove_count = log_files.len() - self.max_log_files;

        for (_, path) in log_files.into_iter().take(remove_count) {
            let _ = fs::remove_file(path);
        }
    }
}

impl Write for DailyFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.rotate_if_needed()?;
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
