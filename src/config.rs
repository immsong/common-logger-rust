use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default)]
pub enum LogTimeZone {
    #[default]
    Local,
    Utc,
}

#[derive(Clone, Debug)]
pub struct LoggerConfig {
    pub log_dir: PathBuf,
    pub filename_prefix: String,
    pub max_log_files: usize,
    pub console_filter: String,
    pub file_filter: String,
    pub time_zone: LogTimeZone,
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
        }
    }
}
