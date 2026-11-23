use std::collections::VecDeque;
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

const MAX_RECORDS: usize = 10_000;
static LOGGER: MemoryLogger = MemoryLogger::new();

#[derive(Default)]
struct LogBuffer {
    records: VecDeque<String>,
    version: u64,
}

struct MemoryLogger {
    buffer: Mutex<LogBuffer>,
}

impl MemoryLogger {
    const fn new() -> Self {
        Self {
            buffer: Mutex::new(LogBuffer {
                records: VecDeque::new(),
                version: 0,
            }),
        }
    }

    fn snapshot(&self, last_version: Option<u64>) -> Option<(u64, Vec<String>)> {
        let buffer = self
            .buffer
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if last_version == Some(buffer.version) {
            return None;
        }
        Some((buffer.version, buffer.records.iter().cloned().collect()))
    }
}

impl Log for MemoryLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= LevelFilter::Debug
            && (metadata.target() == "clausewitz_viewer"
                || metadata.target().starts_with("clausewitz_viewer::"))
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let line = format!(
            "[{}] {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
        let mut buffer = self
            .buffer
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if buffer.records.len() == MAX_RECORDS {
            buffer.records.pop_front();
        }
        buffer.records.push_back(line);
        buffer.version = buffer.version.wrapping_add(1);
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(LevelFilter::Debug);
    Ok(())
}

pub fn snapshot(last_version: Option<u64>) -> Option<(u64, Vec<String>)> {
    LOGGER.snapshot(last_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;

    fn write(logger: &MemoryLogger, level: Level, target: &str, message: &str) {
        logger.log(
            &Record::builder()
                .level(level)
                .target(target)
                .args(format_args!("{message}"))
                .build(),
        );
    }

    #[test]
    fn filters_by_level_and_application_target() {
        let logger = MemoryLogger::new();
        for level in [Level::Error, Level::Warn, Level::Info, Level::Debug] {
            write(&logger, level, "clausewitz_viewer::game", "kept");
        }
        write(&logger, Level::Trace, "clausewitz_viewer::game", "ignored");
        write(&logger, Level::Error, "other_crate", "ignored");
        write(&logger, Level::Info, "clausewitz_viewer_extra", "ignored");

        let (version, records) = logger.snapshot(None).unwrap();
        assert_eq!(records.len(), 4);
        assert!(records.iter().all(|record| record.ends_with("kept")));
        assert!(logger.snapshot(Some(version)).is_none());
    }

    #[test]
    fn installed_logger_captures_facade_messages() {
        init().unwrap();
        log::debug!("logger integration test message");
        log::trace!("logger integration trace message");

        let (_, records) = snapshot(None).unwrap();
        assert!(records
            .iter()
            .any(|record| record.contains("logger integration test message")));
        assert!(!records
            .iter()
            .any(|record| record.contains("logger integration trace message")));
    }

    #[test]
    fn retains_the_latest_records_in_order() {
        let logger = MemoryLogger::new();
        for index in 0..MAX_RECORDS + 2 {
            write(
                &logger,
                Level::Debug,
                "clausewitz_viewer",
                &index.to_string(),
            );
        }

        let (_, records) = logger.snapshot(None).unwrap();
        assert_eq!(records.len(), MAX_RECORDS);
        assert!(records.first().unwrap().ends_with(": 2"));
        assert!(records.last().unwrap().ends_with(": 10001"));
    }
}
