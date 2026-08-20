use crate::LoggerConfig;
use crate::backup::backup_old_logs;
use std::io;
use std::thread;
use std::time::Duration;

const MIN_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(10);

pub(crate) fn start(config: LoggerConfig) -> io::Result<()> {
    let interval = if config.maintenance_interval < MIN_MAINTENANCE_INTERVAL {
        MIN_MAINTENANCE_INTERVAL
    } else {
        config.maintenance_interval
    };

    thread::Builder::new()
        .name("log-maintenance".into())
        .spawn(move || {
            loop {
                thread::sleep(interval);

                match backup_old_logs(&config) {
                    Ok(count) if count > 0 => {
                        tracing::info!(count, "old log backup completed");
                    }
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            "old log backup failed"
                        );
                    }
                    _ => {}
                }
            }
        })?;

    Ok(())
}
