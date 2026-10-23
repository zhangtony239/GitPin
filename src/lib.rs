//! Shared implementation for the `git-pin` and `git-unpin` commands.

/// Package version injected by Cargo at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod launcher;
#[cfg(target_os = "macos")]
pub mod macos_launcher;
pub mod platform;
pub mod repo;
