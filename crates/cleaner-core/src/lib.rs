//! # cleaner-core
//!
//! Domain model, use cases (services) and ports of macOS Cleaner.
//!
//! This crate performs no I/O and knows nothing about Tauri, SQLite or HTTP: every external
//! capability is a trait in [`ports`], implemented by adapters in `cleaner-infra`
//! (hexagonal architecture, Constitution II).

pub mod analysis;
pub mod cancel;
pub mod domain;
pub mod error;
pub mod policy;
pub mod ports;
pub mod services;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
