//! Ports (hexagonal architecture): traits the core depends on, implemented by adapters.

pub mod filesystem;
pub mod repository;

/// Source of the current time, injectable so tests are deterministic.
pub trait Clock: Send + Sync {
    /// Unix epoch seconds, UTC.
    fn now(&self) -> i64;
}
