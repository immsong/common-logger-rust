# common-logger

Common Rust logger with tracing, daily log rotation, gzip backup, and periodic maintenance.

Desktop과 Android 환경에서 공통으로 사용할 수 있는 `tracing` 기반 logger crate입니다.

## Features

- `tracing` 기반 console / file logging
- Console / file log filter 개별 설정
- Local / UTC time zone 지원
- 날짜별 log file rotation
- 이전 날짜 log file 자동 gzip 압축
- `backup/` 디렉터리 자동 관리
- 주기적인 log maintenance
- 최대 log file 개수 제한
- Backup된 파일을 보관 기간 또는 용량 조건으로 자동 관리
- Desktop / Android 공통 사용

## Log Structure

기본 설정에서는 다음과 같은 구조로 log file이 생성됩니다.

```text
logs/
├─ app.2026-08-20.log
└─ backup/
   ├─ app.2026-08-18.log.gz
   └─ app.2026-08-19.log.gz
```

현재 날짜의 log는 `.log` 파일로 유지되며, 이전 날짜의 log는 `backup/` 디렉터리에 gzip 형식으로 압축됩니다.

압축이 정상적으로 완료된 경우에만 원본 `.log` 파일을 삭제합니다.

## Installation

Git repository를 Cargo dependency로 추가합니다.

```toml
[dependencies]
common-logger = { git = "https://github.com/immsong/common-logger-rust.git", tag = "v0.1.0" }
tracing = "0.1"
```

개발 중 local repository를 사용할 경우:

```toml
[dependencies]
common-logger = { path = "../common-logger-rust" }
tracing = "0.1"
```

## Usage

기본 설정:

```rust
use common_logger::{LoggerConfig, initialize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize(LoggerConfig::default())?;

    tracing::info!("Application started");

    Ok(())
}
```

기본 log directory는 다음과 같습니다.

```text
./logs
```

## Configuration

```rust
use std::path::PathBuf;
use std::time::Duration;

use common_logger::{
    LogTimeZone,
    LoggerConfig,
    initialize,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize(LoggerConfig {
        log_dir: PathBuf::from("logs"),
        filename_prefix: "sensor-studio-core".into(),
        max_log_files: 30,
        console_filter: "debug".into(),
        file_filter: "info".into(),
        time_zone: LogTimeZone::Local,
        maintenance_interval: Duration::from_secs(60 * 60),
        backup_retention_days: Some(90),
        max_backup_size_bytes: Some(1024 * 1024 * 1024),
    })?;

    tracing::info!("Application started");

    Ok(())
}
```

### LoggerConfig

| Field | Default | Description |
| --- | --- | --- |
| `log_dir` | `logs` | Log file 저장 디렉터리 |
| `filename_prefix` | `app` | Log file 이름 prefix |
| `max_log_files` | `30` | 유지할 `.log` file 최대 개수 |
| `console_filter` | `debug` | Console log filter |
| `file_filter` | `info` | File log filter |
| `time_zone` | `Local` | Timestamp 및 daily rotation 기준 |
| `maintenance_interval` | `1 hour` | Backup maintenance 실행 주기 |
| `backup_retention_days` | `None` | Backup log 보관 기간. 설정된 기간보다 오래된 `.gz` 삭제 |
| `max_backup_size_bytes` | `None` | Backup 전체 최대 용량. 초과 시 오래된 `.gz`부터 삭제 |

`maintenance_interval`이 10초보다 짧게 설정된 경우 최소 10초로 동작합니다.

## Time Zone

Local time과 UTC를 지원합니다.

```rust
use common_logger::LogTimeZone;
```

Local time:

```rust
time_zone: LogTimeZone::Local,
```

UTC:

```rust
time_zone: LogTimeZone::Utc,
```

선택한 time zone은 다음 항목에 동일하게 적용됩니다.

- Log timestamp
- Log file date
- Daily rotation
- Backup 대상 날짜 판단

## Daily Rotation

Log file은 날짜별로 생성됩니다.

```text
sensor-studio-core.2026-08-19.log
sensor-studio-core.2026-08-20.log
```

`Local`을 사용하면 local midnight 기준으로 rotation되고, `Utc`를 사용하면 UTC midnight 기준으로 rotation됩니다.

## Backup

현재 날짜보다 이전인 log file은 자동으로 gzip 압축됩니다.

```text
logs/
└─ backup/
   └─ sensor-studio-core.2026-08-19.log.gz
```

Backup은 안전하게 다음 순서로 처리됩니다.

```text
.log
  ↓
.log.gz.tmp
  ↓
gzip 완료
  ↓
.log.gz
  ↓
원본 .log 삭제
```

압축 또는 원본 삭제에 실패하면 원본 log를 유지하여 이후 maintenance에서 다시 처리할 수 있도록 합니다.

## Maintenance

Logger 초기화 시 이전 log를 한 번 확인하고, 이후 설정된 주기마다 backup maintenance를 수행합니다.

기본 주기:

```text
1 hour
```

최소 주기:

```text
10 seconds
```

## Backup Retention

Backup log 삭제 정책은 선택적으로 설정할 수 있습니다.

기본값은 두 옵션 모두 `None`이며, 이 경우 backup log를 자동으로 삭제하지 않습니다.

### Retention by Age

```rust
backup_retention_days: Some(90),
max_backup_size_bytes: None,
```
90일보다 오래된 backup log를 삭제합니다.

### Retention by Size

```rust
backup_retention_days: None,
max_backup_size_bytes: Some(1024 * 1024 * 1024),
```
Backup 전체 크기가 1 GiB를 초과하면 가장 오래된 log부터 삭제하여 설정된 크기 이하로 유지합니다.

### Retention by Age and Size

두 옵션을 동시에 사용할 수도 있습니다.

```rust
backup_retention_days: Some(90),
max_backup_size_bytes: Some(1024 * 1024 * 1024),
```

이 경우 다음 순서로 적용됩니다.

  1. 보관 기간을 초과한 backup log 삭제
  2. 남은 backup log의 전체 크기 확인
  3. 최대 크기를 초과하면 가장 오래된 log부터 추가 삭제

### Disable Retention

```rust
backup_retention_days: None,
max_backup_size_bytes: None,
```

Backup log를 자동으로 삭제하지 않습니다.

## Android

`common-logger`는 Android target으로 빌드되는 Rust library에서도 사용할 수 있습니다.

Kotlin/Java 기반 Android application에서 사용할 경우, `common-logger`를 포함한 Rust code를 shared library (`.so`)로 빌드하고 JNI/FFI를 통해 Rust code를 호출해야 합니다.

`common-logger` 자체는 JNI/FFI interface를 제공하지 않으며, Android application과 Rust library 사이의 interface는 사용하는 application에서 구현해야 합니다.

예:

```text
Android Application
    │
    │ JNI / FFI
    ▼
Rust Shared Library (.so)
    │
    └─ common-logger
```

Kotlin:
```kotlin
System.loadLibrary("sensor_studio_core")

private external fun nativeStart(
    filesDir: String,
    logDir: String
)
```

Rust JNI entry point에서 전달받은 log directory를 `LoggerConfig`에 설정할 수 있습니다.

Rust:
```rust
initialize(LoggerConfig {
    log_dir: PathBuf::from(log_dir),
    filename_prefix: "sensor-studio-core".into(),
    ..Default::default()
})?;
```

Android에서는 앱이 write 가능한 절대 경로를 `log_dir`로 전달해야 합니다.

예를 들어 app-specific external storage를 사용할 수 있습니다.

`/storage/emulated/0/Android/data/com.example.app/files/logs`

Kotlin:
```kotlin
val externalFilesDir = requireNotNull(
    getExternalFilesDir(null)
)

val logDir = File(
    externalFilesDir,
    "logs"
).absolutePath
```

`common-logger` 자체는 Android API에 의존하지 않으며, `.so` 빌드, JNI/FFI interface, 저장 경로 결정은 사용하는 application에서 처리합니다.

## Notes

- `initialize()`는 process 내에서 한 번만 tracing subscriber를 등록합니다.
- File logger의 `WorkerGuard`는 process가 종료될 때까지 유지됩니다.
- `max_log_files`는 backup 실패 등의 상황에서 오래된 `.log` 파일이 계속 쌓이는 것을 방지하기 위한 safety limit입니다.
- Backup된 `.gz` 파일은 `max_log_files` 제한 대상에 포함되지 않습니다.
- Backup `.gz` 파일은 `backup_retention_days`와 `max_backup_size_bytes`를 통해 별도로 관리할 수 있습니다.
- 두 backup retention 옵션을 모두 설정하면 기간 정책을 먼저 적용한 뒤 용량 정책을 적용합니다.


## License

This project is licensed under the MIT License.
