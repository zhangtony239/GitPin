//! Native launcher backend selection.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::LinuxBackend;

#[cfg(target_os = "linux")]
pub type NativeBackend = LinuxBackend;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::WindowsBackend;

#[cfg(target_os = "windows")]
pub type NativeBackend = WindowsBackend;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOsBackend;

#[cfg(target_os = "macos")]
pub type NativeBackend = MacOsBackend;
