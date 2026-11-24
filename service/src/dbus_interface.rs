//! Session D-Bus service `io.github.vcanonici.Furigana`, interface version 1.
//!
//! Methods return JSON strings (shapes in `protocol.rs`) so the contract can grow
//! without D-Bus signature churn. Analysed text is never logged.

use std::time::Instant;

use zbus::{connection, fdo, interface};

use crate::analysis::{AnalysisError, Analyzer};
use crate::event_log::{Status, log_event};
use crate::protocol::Health;

/// Well-known bus name.
pub const BUS_NAME: &str = "io.github.vcanonici.Furigana";
/// Object path of the single service object.
pub const OBJECT_PATH: &str = "/io/github/vcanonici/Furigana";
/// Interface name, versioned by its suffix.
pub const INTERFACE: &str = "io.github.vcanonici.Furigana1";

/// The served object; owns the analyzer and a few counters for `Health`.
pub struct FuriganaService {
    analyzer: Analyzer,
    reload: Box<dyn Fn() -> Result<Analyzer, String> + Send + Sync>,
    started: Instant,
    calls: u64,
}

impl FuriganaService {
    /// Wraps an analyzer; `reload` rebuilds it when the user edits overrides.
    pub fn new(analyzer: Analyzer, reload: impl Fn() -> Result<Analyzer, String> + Send + Sync + 'static) -> Self {
        Self { analyzer, reload: Box::new(reload), started: Instant::now(), calls: 0 }
    }

    fn health_report(&self) -> Health {
        let (override_terms, glossary_terms, jmdict) = self.analyzer.glossary().stats();
        Health {
            version: env!("CARGO_PKG_VERSION"),
            interface: INTERFACE,
            uptime_seconds: self.started.elapsed().as_secs(),
            calls: self.calls,
            glossary_terms,
            override_terms,
            jmdict,
        }
    }
}

fn to_json(value: &impl serde::Serialize) -> fdo::Result<String> {
    serde_json::to_string(value).map_err(|error| fdo::Error::Failed(error.to_string()))
}

#[interface(name = "io.github.vcanonici.Furigana1")]
impl FuriganaService {
    /// Analyses a UI string. `context` names the Shell component (for diagnostics only).
    fn analyze(&mut self, text: &str, context: &str) -> fdo::Result<String> {
        self.calls += 1;
        let started = Instant::now();
        let result = self.analyzer.analyze(text).map_err(|error| match error {
            AnalysisError::TooLong => fdo::Error::InvalidArgs(error.to_string()),
            AnalysisError::Morphology(_) => fdo::Error::Failed(error.to_string()),
        });
        let status = if result.is_ok() { Status::Ok } else { Status::Error };
        log_event(
            "dbus",
            "analyze",
            status,
            &[("context", context), ("ms", &started.elapsed().as_millis().to_string())],
        );
        to_json(&*result?)
    }

    /// Looks up one term (dictionary form); returns `null` when unknown.
    fn lookup(&mut self, term: &str, reading: &str) -> fdo::Result<String> {
        self.calls += 1;
        let reading = (!reading.is_empty()).then_some(reading);
        to_json(&self.analyzer.glossary().lookup(term, reading))
    }

    /// Service status for the Settings panel and `gnome-furigana doctor`.
    fn health(&mut self) -> fdo::Result<String> {
        self.calls += 1;
        to_json(&self.health_report())
    }

    /// Reloads glossaries (after `overrides.tsv` changes) and clears the cache.
    fn reload(&mut self) -> fdo::Result<String> {
        self.analyzer = (self.reload)().map_err(fdo::Error::Failed)?;
        log_event("dbus", "reload", Status::Ok, &[]);
        self.health()
    }
}

/// Owns the bus name and serves until the process is stopped.
///
/// # Errors
/// Fails when the session bus is unavailable or the name is already owned.
pub fn serve(service: FuriganaService) -> zbus::Result<()> {
    zbus::block_on(async {
        let _connection = connection::Builder::session().name(BUS_NAME).serve_at(OBJECT_PATH, service).build().await?;
        log_event("dbus", "serve", Status::Ok, &[("name", BUS_NAME)]);
        std::future::pending::<()>().await;
        Ok(())
    })
}
