//! Shared implementation for the `git-pin` and `git-unpin` commands.

pub mod app;
pub mod cli;
pub mod error;
pub mod launcher;
#[cfg(target_os = "macos")]
pub mod macos_launcher;
pub mod platform;
pub mod repo;
