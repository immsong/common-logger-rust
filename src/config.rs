use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, NaiveDate, Utc};

#[derive(Clone, Copy, Debug, Default)]
pub enum LogTimeZone {
    #[default]
    Local,
    Utc,
}

impl LogTimeZone {
    pub(crate) fn current_date(self) -> NaiveDate {
        match self {
            Self::Local => Local::now().date_naive(),
            Self::Utc => Utc::now().date_naive(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoggerConfig {
    pub log_dir: PathBuf,
    pub filename_prefix: String,
    pub max_log_files: usize,
    pub console_filter: String,
    pub file_filter: String,
    pub time_zone: LogTimeZone,
    pub maintenance_interval: Duration,
    pub backup_retention_days: Option<u64>,
    pub max_backup_size_bytes: Option<u64>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_dir: "logs".into(),
            filename_prefix: "app".into(),
            max_log_files: 30,
            console_filter: "debug".into(),
            file_filter: "info".into(),
            time_zone: LogTimeZone::Local,
            maintenance_interval: Duration::from_secs(60 * 60),
            backup_retention_days: None,
            max_backup_size_bytes: None,
        }
    }
}
