//! IPC command handlers, grouped by user story.

// Tauri injects `State` and deserialised arguments by value; that is the framework idiom.
#![allow(clippy::needless_pass_by_value)]

pub mod catalog;
pub mod scan;
pub mod system;
