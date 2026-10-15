//! # cleaner-infra
//!
//! Adapters implementing the `cleaner-core` ports: filesystem scanning, SQLite persistence,
//! AI providers, secret stores and system integrations (macOS, Windows, Linux).

pub mod ai;
pub mod db;
pub mod fs;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod secrets;
pub mod system;

/// System clock adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl cleaner_core::ports::Clock for SystemClock {
    fn now(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
    }
}
