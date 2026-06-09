//! In-memory event ring buffer for the dashboard event log.
//!
//! Mirrors Python's `event_logger.EventLogger` format so the
//! dashboard `index.html` renders events without any JS changes.

use serde::Serialize;
use std::collections::VecDeque;
use parking_lot::Mutex;

const MAX_EVENTS: usize = 5000;

/// A single dashboard event entry.
#[derive(Debug, Clone, Serialize)]
pub struct EventEntry {
    pub timestamp_us: u64,
    pub data: EventData,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventData {
    pub s: String, // source
    pub t: String, // type
    pub d: serde_json::Value, // details
}

/// Thread-safe ring buffer.
pub struct EventBuffer {
    buf: Mutex<VecDeque<EventEntry>>,
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self {
            buf: Mutex::new(VecDeque::with_capacity(MAX_EVENTS)),
        }
    }
}

impl EventBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new event. Never blocks — if the mutex is poisoned the event is dropped.
    pub fn push(&self, source: &str, typ: &str, data: impl Serialize) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let d = match serde_json::to_value(&data) {
            Ok(v) => v,
            Err(_) => serde_json::Value::Null,
        };

        let entry = EventEntry {
            timestamp_us: ts,
            data: EventData {
                s: source.to_string(),
                t: typ.to_string(),
                d,
            },
        };

        if let Some(mut buf) = self.buf.try_lock() {
            if buf.len() >= MAX_EVENTS {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }

    /// Return a snapshot of all events.
    pub fn snapshot(&self) -> Vec<EventEntry> {
        match self.buf.try_lock() {
            Some(buf) => buf.iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// Export as newline-delimited JSON.
    pub fn export_ndjson(&self) -> String {
        let events = self.snapshot();
        events
            .into_iter()
            .filter_map(|e| serde_json::to_string(&e).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
