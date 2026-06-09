//! Async logging worker — never blocks the hot path.
//!
//! The executor and feeds send `LogEvent` messages through a bounded
//! flume channel. This worker receives them on a blocking thread and
//! writes structured output.

use flume::Receiver;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::signals::{LogEvent, LogLevel};

/// Start the logging worker on a dedicated blocking thread.
/// Returns immediately. The worker runs until the channel closes.
pub fn start(rx: Receiver<LogEvent>, log_dir: Option<&str>) {
    let dir = log_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("logs"));

    let log_path = dir.clone();

    std::thread::Builder::new()
        .name("logger-worker".into())
        .spawn(move || {
            run_worker(rx, &dir);
        })
        .expect("failed to spawn logger thread");

    tracing::info!(dir = %log_path.display(), "logger_worker_started");
}

fn run_worker(rx: Receiver<LogEvent>, dir: &PathBuf) {
    let _ = std::fs::create_dir_all(dir);

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("bot.log"))
        .map(BufWriter::new);

    let mut file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!("logger: cannot open log file: {e}");
            return;
        }
    };

    while let Ok(event) = rx.recv() {
        let level = match event.level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        };

        let ts = event.timestamp_ms;
        let mut line = format!("{ts} [{level}] {} {}", event.module, event.message);

        for (k, v) in &event.fields {
            line.push_str(&format!(" {k}={v}"));
        }

        let _ = writeln!(file, "{line}");
    }

    let _ = file.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::log_channel;

    #[test]
    fn test_logger_receives_and_writes() {
        let (tx, rx) = log_channel();
        let tmp = std::env::temp_dir().join("polymarket_test_log");
        let _ = std::fs::create_dir_all(&tmp);

        // Send some events
        tx.send(LogEvent {
            level: LogLevel::Info,
            module: "test",
            message: "hello".into(),
            fields: vec![("key".into(), "val".into())],
            timestamp_ms: 12345,
        })
        .unwrap();

        tx.send(LogEvent {
            level: LogLevel::Error,
            module: "test",
            message: "error!".into(),
            fields: vec![],
            timestamp_ms: 12346,
        })
        .unwrap();

        drop(tx); // close channel

        start(rx, Some(tmp.to_str().unwrap()));

        // Wait briefly for worker to process
        std::thread::sleep(std::time::Duration::from_millis(200));

        let log_path = tmp.join("bot.log");
        assert!(log_path.exists());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("hello"));
        assert!(content.contains("error!"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
