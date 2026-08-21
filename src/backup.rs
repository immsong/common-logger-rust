use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use chrono::{Days, NaiveDate};
use flate2::Compression;
use flate2::write::GzEncoder;

use crate::LoggerConfig;

struct BackupLogFile {
    date: NaiveDate,
    path: PathBuf,
    size: u64,
}

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
        if let Err(error) = fs::remove_file(source_path) {
            let _ = fs::remove_file(&backup_path);
            return Err(error);
        }

        Ok(true)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

pub(crate) fn prune_backup_logs(config: &LoggerConfig) -> io::Result<usize> {
    let mut removed_count = 0;

    if let Some(days) = config.backup_retention_days {
        removed_count += prune_backup_by_days(config, days)?;
    }

    if let Some(max_size) = config.max_backup_size_bytes {
        removed_count += prune_backup_by_size(config, max_size)?;
    }

    Ok(removed_count)
}

fn prune_backup_by_days(config: &LoggerConfig, retention_days: u64) -> io::Result<usize> {
    let today = config.time_zone.current_date();

    let Some(cutoff_date) = today.checked_sub_days(Days::new(retention_days)) else {
        return Ok(0);
    };

    let backup_files = collect_backup_logs(config)?;
    let mut removed_count = 0;

    for file in backup_files {
        if file.date < cutoff_date {
            fs::remove_file(&file.path)?;
            removed_count += 1;
        }
    }

    Ok(removed_count)
}

fn prune_backup_by_size(config: &LoggerConfig, max_size: u64) -> io::Result<usize> {
    let mut backup_files = collect_backup_logs(config)?;

    backup_files.sort_by_key(|file| file.date);

    let mut total_size = backup_files.iter().map(|file| file.size).sum::<u64>();

    let mut removed_count = 0;

    for file in backup_files {
        if total_size <= max_size {
            break;
        }

        fs::remove_file(&file.path)?;

        total_size = total_size.saturating_sub(file.size);
        removed_count += 1;
    }

    Ok(removed_count)
}

fn collect_backup_logs(config: &LoggerConfig) -> io::Result<Vec<BackupLogFile>> {
    let backup_dir = config.log_dir.join("backup");

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let prefix = format!("{}.", config.filename_prefix);
    let mut files = Vec::new();

    for entry in fs::read_dir(&backup_dir)? {
        let Ok(entry) = entry else {
            continue;
        };

        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if !metadata.is_file() {
            continue;
        }

        let filename = entry.file_name();

        let Some(filename) = filename.to_str() else {
            continue;
        };

        let Some(date) = filename
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(".log.gz"))
        else {
            continue;
        };

        let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
            continue;
        };

        files.push(BackupLogFile {
            date,
            path: entry.path(),
            size: metadata.len(),
        });
    }

    Ok(files)
}
