//! Structured one-line JSON events on stderr (collected by the systemd journal).

use std::time::{SystemTime, UNIX_EPOCH};

/// Outcome of a logged step.
#[derive(Debug, Clone, Copy)]
pub enum Status {
    /// Step succeeded.
    Ok,
    /// Step failed.
    Error,
}

/// Writes `{"ts":…,"module":…,"event":…,"status":…,<fields>}` to stderr.
/// Callers must not pass analysed UI text or other private content as fields.
pub fn log_event(module: &str, event: &str, status: Status, fields: &[(&str, &str)]) {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_millis());
    let mut record = serde_json::Map::new();
    record.insert("ts_ms".into(), timestamp.to_string().into());
    record.insert("module".into(), module.into());
    record.insert("event".into(), event.into());
    record.insert("status".into(), if matches!(status, Status::Ok) { "ok" } else { "error" }.into());
    for (key, value) in fields {
        record.insert((*key).into(), (*value).into());
    }
    eprintln!("{}", serde_json::Value::Object(record));
}
