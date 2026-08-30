//! Platform-independent launcher contracts.

use std::path::{Path, PathBuf};
use std::{fmt, io};

use crate::error::AppError;
use crate::repo::Repository;

/// Root directory selected for launcher storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherRoot(PathBuf);

impl LauncherRoot {
    /// Used by native production constructors after resolving the system path.
    pub fn system(path: PathBuf) -> Self {
        Self(path)
    }

    /// Explicit injection point for isolated integration tests.
    #[cfg(test)]
    pub(crate) fn for_test(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Metadata read from one launcher managed by Git Pin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedLauncher {
    pub name: String,
    pub root: PathBuf,
    pub ide_executable: PathBuf,
    pub path: PathBuf,
}

/// Failure to parse one launcher candidate while continuing enumeration.
#[derive(Clone, Debug)]
pub struct LauncherEnumerationError {
    pub path: PathBuf,
    detail: String,
}

impl LauncherEnumerationError {
    pub fn new(path: PathBuf, detail: impl Into<String>) -> Self {
        Self {
            path,
            detail: detail.into(),
        }
    }

    pub fn from_io(path: PathBuf, action: &str, error: io::Error) -> Self {
        Self::new(path, format!("{action}: {error}"))
    }
}

impl fmt::Display for LauncherEnumerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "launcher candidate '{}': {}",
            self.path.display(),
            self.detail
        )
    }
}

/// One independently parsed entry from a platform launcher directory.
pub type LauncherEnumerationItem = Result<ManagedLauncher, LauncherEnumerationError>;

/// Result of inspecting the platform location for a repository name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LauncherInspection {
    Missing,
    Managed(ManagedLauncher),
    Foreign { path: PathBuf },
}

/// Result of an atomic create attempt after an initially missing inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreateOutcome {
    Created(ManagedLauncher),
    Occupied(LauncherInspection),
}

/// Native behavior required by the shared application orchestration.
pub trait LauncherBackend {
    /// The isolated or system launcher root owned by this backend instance.
    fn launcher_root(&self) -> &LauncherRoot;

    /// Inspects the exact launcher slot associated with `name`.
    fn inspect(&self, name: &str) -> Result<LauncherInspection, AppError>;

    /// Enumerates managed launchers, preserving per-candidate parsing failures.
    fn enumerate(&self) -> Result<Vec<LauncherEnumerationItem>, AppError>;

    /// Creates and atomically commits a launcher into a previously missing slot.
    fn create(
        &self,
        repository: &Repository,
        ide_executable: &Path,
    ) -> Result<CreateOutcome, AppError>;

    /// Removes exactly the launcher that was previously inspected and verified.
    fn remove(&self, launcher: &ManagedLauncher) -> Result<(), AppError>;
}

#[cfg(test)]
mod tests {
    use super::LauncherRoot;
    use std::path::PathBuf;

    #[test]
    fn test_launcher_root_is_explicitly_injected() {
        let path = PathBuf::from("isolated-launchers");
        let root = LauncherRoot::for_test(path.clone());
        assert_eq!(root.as_path(), path);
    }
}
