use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use flate2::Compression;
use flate2::write::GzEncoder;

use crate::LoggerConfig;

pub(crate) fn backup_old_logs(config: &LoggerConfig) -> io::Result<usize> {
    let today = config.time_zone.current_date();

    let mut log_files = collect_old_logs(config, today)?;

    if log_files.is_empty() {
        return Ok(0);
    }

    log_files.sort_by_key(|(date, _)| *date);

    let backup_dir = config.log_dir.join("backup");
    fs::create_dir_all(&backup_dir)?;

    let mut backup_count = 0;
    let mut first_error = None;

    for (_, path) in log_files {
        match backup_log_file(&path, &backup_dir) {
            Ok(true) => {
                backup_count += 1;
            }
            Ok(false) => {}
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(io::Error::new(
                        error.kind(),
                        format!("failed to backup {}: {error}", path.display()),
                    ));
                }
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    Ok(backup_count)
}

fn collect_old_logs(
    config: &LoggerConfig,
    today: NaiveDate,
) -> io::Result<Vec<(NaiveDate, PathBuf)>> {
    let prefix = format!("{}.", config.filename_prefix);

    let log_files = fs::read_dir(&config.log_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;

            if !file_type.is_file() {
                return None;
            }

            let filename = entry.file_name();
            let filename = filename.to_str()?;

            let date = filename.strip_prefix(&prefix)?.strip_suffix(".log")?;

            let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;

            if date >= today {
                return None;
            }

            Some((date, entry.path()))
        })
        .collect();

    Ok(log_files)
}

fn backup_log_file(source_path: &Path, backup_dir: &Path) -> io::Result<bool> {
    let filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::other("invalid log filename"))?;

    let backup_path = backup_dir.join(format!("{filename}.gz"));
    let temp_path = backup_dir.join(format!("{filename}.gz.tmp"));

    // Keep the source if a backup already exists.
    if backup_path.exists() {
        return Ok(false);
    }

    let result = (|| {
        let mut source = File::open(source_path)?;
        let temp_file = File::create(&temp_path)?;

        let mut encoder = GzEncoder::new(temp_file, Compression::default());

        io::copy(&mut source, &mut encoder)?;

        let output = encoder.finish()?;
        output.sync_all()?;

        fs::rename(&temp_path, &backup_path)?;

        // Remove the source only after the backup is complete.
        fs::remove_file(source_path)?;

        Ok(true)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}
