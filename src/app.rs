//! Application orchestration shared by both command-line entry points.

use std::ffi::OsStr;
use std::path::Path;

use crate::cli::{Invocation, PIN_HELP};
use crate::error::AppError;
use crate::launcher::{CreateOutcome, LauncherBackend, LauncherInspection, ManagedLauncher};
use crate::platform::NativeBackend;
use crate::repo::{check_root, launcher_name, paths_equivalent, Platform, Repository, RootStatus};

/// Successful states returned by pin orchestration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinOutcome {
    Created(ManagedLauncher),
    AlreadyPinned(ManagedLauncher),
}

/// A resolved request to remove a launcher by repository identity or exact name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnpinTarget {
    Repository(Repository),
    Name(String),
}

/// Successful states returned by unpin orchestration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnpinOutcome {
    Removed(ManagedLauncher),
    AlreadyAbsent,
}

/// One successfully parsed launcher and its repository-root status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherRecord {
    pub launcher: ManagedLauncher,
    pub status: RootStatus,
}

/// Best-effort scan output with independent candidate/check diagnostics.
#[derive(Debug)]
pub struct ScanReport {
    pub records: Vec<LauncherRecord>,
    pub errors: Vec<String>,
}

/// Best-effort prune output; successful removals are never rolled back.
#[derive(Debug)]
pub struct PruneReport {
    pub removed: Vec<ManagedLauncher>,
    pub errors: Vec<String>,
}

/// Enumerates, checks and deterministically sorts all managed launchers.
pub fn scan<B: LauncherBackend>(backend: &B, platform: Platform) -> Result<ScanReport, AppError> {
    let mut records = Vec::new();
    let mut errors = Vec::new();
    for item in backend.enumerate()? {
        match item {
            Ok(launcher) => match check_root(&launcher.root, platform) {
                Ok(status) => records.push(LauncherRecord { launcher, status }),
                Err(error) => errors.push(format!(
                    "could not check launcher '{}' with recorded root '{}': {error}",
                    launcher.name,
                    launcher.root.display()
                )),
            },
            Err(error) => errors.push(error.to_string()),
        }
    }
    records.sort_by(|left, right| {
        let left_name = match platform {
            Platform::Windows => left.launcher.name.to_lowercase(),
            Platform::Linux | Platform::MacOs => left.launcher.name.clone(),
        };
        let right_name = match platform {
            Platform::Windows => right.launcher.name.to_lowercase(),
            Platform::Linux | Platform::MacOs => right.launcher.name.clone(),
        };
        left_name
            .cmp(&right_name)
            .then_with(|| left.launcher.name.cmp(&right.launcher.name))
    });
    Ok(ScanReport { records, errors })
}

/// Lists all managed launchers without modifying them.
pub fn list<B: LauncherBackend>(backend: &B, platform: Platform) -> Result<ScanReport, AppError> {
    scan(backend, platform)
}

/// Removes only launchers whose repository root remains invalid on a pre-delete recheck.
pub fn prune<B: LauncherBackend>(backend: &B, platform: Platform) -> Result<PruneReport, AppError> {
    let scan = scan(backend, platform)?;
    let mut removed = Vec::new();
    let mut errors = scan.errors;

    for record in scan.records {
        if !matches!(record.status, RootStatus::Invalid(_)) {
            continue;
        }
        match check_root(&record.launcher.root, platform) {
            Ok(RootStatus::Valid) => continue,
            Ok(RootStatus::Invalid(_)) => match backend.remove(&record.launcher) {
                Ok(()) => removed.push(record.launcher),
                Err(error) => errors.push(format!(
                    "could not prune launcher '{}' with recorded root '{}' at '{}': {error}",
                    record.launcher.name,
                    record.launcher.root.display(),
                    record.launcher.path.display()
                )),
            },
            Err(error) => errors.push(format!(
                "could not recheck launcher '{}' with recorded root '{}' before pruning: {error}",
                record.launcher.name,
                record.launcher.root.display()
            )),
        }
    }

    Ok(PruneReport { removed, errors })
}

fn batch_error(action: &str, errors: Vec<String>) -> Result<(), AppError> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::failure(format!(
            "{action} completed with {} error(s):\n{}",
            errors.len(),
            errors
                .into_iter()
                .map(|error| format!("- {error}"))
                .collect::<Vec<_>>()
                .join("\n")
        )))
    }
}

/// Creates or confirms one platform launcher without overwriting conflicts.
pub fn pin<B: LauncherBackend>(
    backend: &B,
    repository: &Repository,
    platform: Platform,
) -> Result<PinOutcome, AppError> {
    let vscode = backend.vscode_executable().map_err(|error| {
        AppError::failure(format!(
            "could not pin repository '{}' because Visual Studio Code is unavailable: {error}",
            repository.root().display()
        ))
    })?;

    match backend.inspect(repository.name()).map_err(|error| {
        AppError::failure(format!(
            "could not inspect launcher '{}' before pinning '{}': {error}",
            repository.name(),
            repository.root().display()
        ))
    })? {
        LauncherInspection::Missing => {
            match backend.create(repository, &vscode).map_err(|error| {
                AppError::failure(format!(
                    "could not atomically create launcher '{}' for '{}': {error}",
                    repository.name(),
                    repository.root().display()
                ))
            })? {
                CreateOutcome::Created(launcher) => Ok(PinOutcome::Created(launcher)),
                CreateOutcome::Occupied(inspection) => {
                    resolve_existing(repository, platform, inspection)
                }
            }
        }
        inspection => resolve_existing(repository, platform, inspection),
    }
}

fn resolve_existing(
    repository: &Repository,
    platform: Platform,
    inspection: LauncherInspection,
) -> Result<PinOutcome, AppError> {
    match inspection {
        LauncherInspection::Managed(launcher)
            if paths_equivalent(repository.root(), &launcher.root, platform) =>
        {
            Ok(PinOutcome::AlreadyPinned(launcher))
        }
        LauncherInspection::Managed(launcher) => Err(AppError::failure(format!(
            "cannot pin repository '{}': launcher '{}' already points to '{}'",
            repository.root().display(),
            repository.name(),
            launcher.root.display()
        ))),
        LauncherInspection::Foreign { path } => Err(AppError::failure(format!(
            "cannot pin repository '{}': launcher path '{}' is not managed by git-pin",
            repository.root().display(),
            path.display()
        ))),
        LauncherInspection::Missing => Err(AppError::failure(format!(
            "could not create launcher '{}' for '{}': atomic commit reported an empty slot",
            repository.name(),
            repository.root().display()
        ))),
    }
}

/// Resolves the V1 unpin ambiguity: existing paths win, otherwise use exact name.
pub fn resolve_unpin_target(
    argument: Option<&OsStr>,
    current_directory: &Path,
    platform: Platform,
) -> Result<UnpinTarget, AppError> {
    match argument {
        None => Repository::discover(current_directory, platform).map(UnpinTarget::Repository),
        Some(argument) => {
            let path = Path::new(argument);
            if path.exists() {
                Repository::discover(path, platform).map(UnpinTarget::Repository)
            } else {
                let name = argument.to_str().ok_or_else(|| {
                    AppError::failure(
                        "unpin name is not valid UTF-8 and does not identify an existing path",
                    )
                })?;
                launcher_name(name, platform).map(UnpinTarget::Name)
            }
        }
    }
}

/// Removes a verified managed launcher, or succeeds when it is already absent.
pub fn unpin<B: LauncherBackend>(
    backend: &B,
    target: &UnpinTarget,
    platform: Platform,
) -> Result<UnpinOutcome, AppError> {
    let name = match target {
        UnpinTarget::Repository(repository) => repository.name(),
        UnpinTarget::Name(name) => name,
    };
    let inspection = backend.inspect(name).map_err(|error| {
        AppError::failure(format!(
            "could not inspect launcher '{name}' before unpinning: {error}"
        ))
    })?;

    match inspection {
        LauncherInspection::Missing => Ok(UnpinOutcome::AlreadyAbsent),
        LauncherInspection::Foreign { path } => Err(AppError::failure(format!(
            "cannot unpin '{name}': launcher path '{}' is not managed by git-pin",
            path.display()
        ))),
        LauncherInspection::Managed(launcher) => {
            if let UnpinTarget::Repository(repository) = target {
                if !paths_equivalent(repository.root(), &launcher.root, platform) {
                    return Err(AppError::failure(format!(
                        "cannot unpin repository '{}': launcher '{}' points to different root '{}'",
                        repository.root().display(),
                        name,
                        launcher.root.display()
                    )));
                }
            }

            backend.remove(&launcher).map_err(|error| {
                AppError::failure(format!(
                    "could not remove launcher '{}' at '{}': {error}",
                    name,
                    launcher.path.display()
                ))
            })?;
            Ok(UnpinOutcome::Removed(launcher))
        }
    }
}

/// Runs one of the public commands.
pub fn run(invocation: Invocation) -> Result<(), AppError> {
    if invocation == Invocation::Help {
        print!("{PIN_HELP}");
        return Ok(());
    }

    let platform = Platform::current();
    let backend = NativeBackend::new()?;

    match invocation {
        Invocation::Pin(argument) => {
            let current_directory = std::env::current_dir().map_err(|error| {
                AppError::failure(format!("could not determine current directory: {error}"))
            })?;
            let input = argument
                .as_deref()
                .map(Path::new)
                .unwrap_or(&current_directory);
            let repository = Repository::discover(input, platform)?;
            match pin(&backend, &repository, platform)? {
                PinOutcome::Created(launcher) => println!(
                    "pinned '{}' at '{}'",
                    repository.root().display(),
                    launcher.path.display()
                ),
                PinOutcome::AlreadyPinned(launcher) => println!(
                    "already pinned '{}' at '{}'",
                    repository.root().display(),
                    launcher.path.display()
                ),
            }
            Ok(())
        }
        Invocation::Unpin(argument) => {
            let current_directory = std::env::current_dir().map_err(|error| {
                AppError::failure(format!("could not determine current directory: {error}"))
            })?;
            let target = resolve_unpin_target(argument.as_deref(), &current_directory, platform)?;
            match unpin(&backend, &target, platform)? {
                UnpinOutcome::Removed(launcher) => {
                    println!("unpinned '{}'", launcher.name)
                }
                UnpinOutcome::AlreadyAbsent => println!("already unpinned"),
            }
            Ok(())
        }
        Invocation::List => {
            let report = list(&backend, platform)?;
            if report.records.is_empty() {
                println!("no pinned repositories");
            } else {
                for record in report.records {
                    match record.status {
                        RootStatus::Valid => println!(
                            "{}\t{}\tvalid",
                            record.launcher.name,
                            record.launcher.root.display()
                        ),
                        RootStatus::Invalid(reason) => println!(
                            "{}\t{}\tinvalid: {}",
                            record.launcher.name,
                            record.launcher.root.display(),
                            reason
                        ),
                    }
                }
            }
            batch_error("list", report.errors)
        }
        Invocation::Prune => {
            let report = prune(&backend, platform)?;
            if report.removed.is_empty() {
                println!("no stale pinned repositories to prune");
            } else {
                for launcher in report.removed {
                    println!(
                        "pruned '{}' at '{}'",
                        launcher.name,
                        launcher.root.display()
                    );
                }
            }
            batch_error("prune", report.errors)
        }
        Invocation::Help => unreachable!("help returns before environment access"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        list, pin, prune, resolve_unpin_target, unpin, PinOutcome, UnpinOutcome, UnpinTarget,
    };
    use crate::error::AppError;
    use crate::launcher::{
        CreateOutcome, LauncherBackend, LauncherEnumerationError, LauncherEnumerationItem,
        LauncherInspection, LauncherRoot, ManagedLauncher,
    };
    use crate::repo::{Platform, Repository};
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "git-pin-app-test-{}-{nonce}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("temporary root must be created");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct FakeBackend {
        root: LauncherRoot,
        inspection: RefCell<LauncherInspection>,
        occupied_on_create: RefCell<Option<LauncherInspection>>,
        fail_vscode: Cell<bool>,
        fail_inspect: Cell<bool>,
        fail_create: Cell<bool>,
        fail_remove: Cell<bool>,
        enumeration: RefCell<Option<Vec<LauncherEnumerationItem>>>,
        fail_remove_names: RefCell<Vec<String>>,
        removed: RefCell<Vec<String>>,
    }

    impl FakeBackend {
        fn new(root: PathBuf) -> Self {
            Self {
                root: LauncherRoot::for_test(root),
                inspection: RefCell::new(LauncherInspection::Missing),
                occupied_on_create: RefCell::new(None),
                fail_vscode: Cell::new(false),
                fail_inspect: Cell::new(false),
                fail_create: Cell::new(false),
                fail_remove: Cell::new(false),
                enumeration: RefCell::new(None),
                fail_remove_names: RefCell::new(Vec::new()),
                removed: RefCell::new(Vec::new()),
            }
        }

        fn launcher(&self, name: &str, root: &Path) -> ManagedLauncher {
            ManagedLauncher {
                name: name.to_owned(),
                root: root.to_owned(),
                path: self.root.as_path().join(name),
            }
        }

        fn set_inspection(&self, inspection: LauncherInspection) {
            *self.inspection.borrow_mut() = inspection;
        }

        fn temp_path(&self, name: &str) -> PathBuf {
            self.root.as_path().join(format!(".{name}.tmp"))
        }

        fn set_enumeration(&self, items: Vec<LauncherEnumerationItem>) {
            *self.enumeration.borrow_mut() = Some(items);
        }
    }

    impl LauncherBackend for FakeBackend {
        fn launcher_root(&self) -> &LauncherRoot {
            &self.root
        }

        fn vscode_executable(&self) -> Result<PathBuf, AppError> {
            if self.fail_vscode.get() {
                Err(AppError::failure("fake VS Code lookup failed"))
            } else {
                Ok(PathBuf::from("fake-code"))
            }
        }

        fn inspect(&self, _name: &str) -> Result<LauncherInspection, AppError> {
            if self.fail_inspect.get() {
                Err(AppError::failure("fake inspect failed"))
            } else {
                Ok(self.inspection.borrow().clone())
            }
        }

        fn enumerate(&self) -> Result<Vec<LauncherEnumerationItem>, AppError> {
            if let Some(items) = self.enumeration.borrow().as_ref() {
                return Ok(items.clone());
            }
            Ok(match self.inspection.borrow().clone() {
                LauncherInspection::Managed(launcher) => vec![Ok(launcher)],
                LauncherInspection::Missing | LauncherInspection::Foreign { .. } => Vec::new(),
            })
        }

        fn create(
            &self,
            repository: &Repository,
            _vscode: &Path,
        ) -> Result<CreateOutcome, AppError> {
            fs::create_dir_all(self.root.as_path())
                .map_err(|error| AppError::failure(error.to_string()))?;
            let temporary = self.temp_path(repository.name());
            fs::write(&temporary, repository.root().as_os_str().as_encoded_bytes())
                .map_err(|error| AppError::failure(error.to_string()))?;

            if self.fail_create.get() {
                let _ = fs::remove_file(&temporary);
                return Err(AppError::failure("fake create failed"));
            }
            if let Some(inspection) = self.occupied_on_create.borrow_mut().take() {
                let _ = fs::remove_file(&temporary);
                return Ok(CreateOutcome::Occupied(inspection));
            }

            let launcher = self.launcher(repository.name(), repository.root());
            fs::rename(&temporary, &launcher.path)
                .map_err(|error| AppError::failure(error.to_string()))?;
            self.set_inspection(LauncherInspection::Managed(launcher.clone()));
            Ok(CreateOutcome::Created(launcher))
        }

        fn remove(&self, launcher: &ManagedLauncher) -> Result<(), AppError> {
            if self.fail_remove.get()
                || self
                    .fail_remove_names
                    .borrow()
                    .iter()
                    .any(|name| name == &launcher.name)
            {
                return Err(AppError::failure("fake remove failed"));
            }
            if launcher.path.exists() {
                fs::remove_file(&launcher.path)
                    .map_err(|error| AppError::failure(error.to_string()))?;
            }
            self.set_inspection(LauncherInspection::Missing);
            self.removed.borrow_mut().push(launcher.name.clone());
            Ok(())
        }
    }

    fn repository(root: &str) -> Repository {
        Repository::fixture(PathBuf::from(root), "project")
    }

    fn initialize_repository(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("init")
            .output()
            .expect("Git must be available in CI");
        assert!(output.status.success());
    }

    #[test]
    fn pin_creates_once_and_is_idempotent_for_the_same_root() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let repository = repository("/work/project");

        assert!(matches!(
            pin(&backend, &repository, Platform::Linux).expect("pin must succeed"),
            PinOutcome::Created(_)
        ));
        assert!(matches!(
            pin(&backend, &repository, Platform::Linux).expect("repeat pin must succeed"),
            PinOutcome::AlreadyPinned(_)
        ));
        assert!(temporary.0.join("project").is_file());
        assert!(!backend.temp_path("project").exists());
    }

    #[test]
    fn pin_rejects_conflicts_and_foreign_artifacts() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let other = backend.launcher("project", Path::new("/other/project"));
        backend.set_inspection(LauncherInspection::Managed(other));
        let error = pin(&backend, &repository("/work/project"), Platform::Linux)
            .expect_err("different roots must conflict");
        assert!(error.to_string().contains("/other/project"));

        let foreign = temporary.0.join("project");
        backend.set_inspection(LauncherInspection::Foreign {
            path: foreign.clone(),
        });
        assert!(pin(&backend, &repository("/work/project"), Platform::Linux).is_err());
        assert!(
            !foreign.exists(),
            "orchestration must not touch foreign paths"
        );
    }

    #[test]
    fn pin_handles_atomic_races_without_leaving_temporary_files() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let repository = repository("/work/project");
        let raced = backend.launcher("project", repository.root());
        *backend.occupied_on_create.borrow_mut() = Some(LauncherInspection::Managed(raced.clone()));

        assert_eq!(
            pin(&backend, &repository, Platform::Linux).expect("same-root race is idempotent"),
            PinOutcome::AlreadyPinned(raced)
        );
        assert!(!backend.temp_path("project").exists());
    }

    #[test]
    fn pin_contextualizes_lookup_inspect_and_create_failures() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let repository = repository("/work/project");

        backend.fail_vscode.set(true);
        assert!(pin(&backend, &repository, Platform::Linux)
            .expect_err("lookup must fail")
            .to_string()
            .contains("Visual Studio Code"));
        backend.fail_vscode.set(false);

        backend.fail_inspect.set(true);
        assert!(pin(&backend, &repository, Platform::Linux)
            .expect_err("inspect must fail")
            .to_string()
            .contains("inspect launcher"));
        backend.fail_inspect.set(false);

        backend.fail_create.set(true);
        assert!(pin(&backend, &repository, Platform::Linux)
            .expect_err("create must fail")
            .to_string()
            .contains("atomically create"));
        assert!(!backend.temp_path("project").exists());
    }

    #[test]
    fn unpin_is_idempotent_and_removes_only_matching_managed_launchers() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let repository = repository("/work/project");
        let target = UnpinTarget::Repository(repository.clone());

        assert_eq!(
            unpin(&backend, &target, Platform::Linux).expect("absence is success"),
            UnpinOutcome::AlreadyAbsent
        );
        let launcher = backend.launcher("project", repository.root());
        fs::write(&launcher.path, "managed").expect("launcher fixture must be written");
        backend.set_inspection(LauncherInspection::Managed(launcher.clone()));
        assert_eq!(
            unpin(&backend, &target, Platform::Linux).expect("unpin must succeed"),
            UnpinOutcome::Removed(launcher.clone())
        );
        assert!(!launcher.path.exists());
    }

    #[test]
    fn unpin_rejects_mismatch_foreign_and_remove_failures() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.clone());
        let target = UnpinTarget::Repository(repository("/work/project"));
        let other = backend.launcher("project", Path::new("/other/project"));
        backend.set_inspection(LauncherInspection::Managed(other));
        assert!(unpin(&backend, &target, Platform::Linux).is_err());

        let foreign = temporary.0.join("project");
        fs::write(&foreign, "foreign").expect("foreign fixture must be written");
        backend.set_inspection(LauncherInspection::Foreign {
            path: foreign.clone(),
        });
        assert!(unpin(&backend, &target, Platform::Linux).is_err());
        assert!(foreign.exists(), "foreign artifact must not be deleted");

        let launcher = backend.launcher("project", Path::new("/work/project"));
        backend.set_inspection(LauncherInspection::Managed(launcher));
        backend.fail_remove.set(true);
        assert!(unpin(&backend, &target, Platform::Linux)
            .expect_err("remove must fail")
            .to_string()
            .contains("could not remove launcher"));
    }

    #[test]
    fn unpin_resolution_prefers_existing_paths_and_falls_back_to_exact_names() {
        let repository_directory = TempDir::new();
        let git_status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repository_directory.0)
            .arg("init")
            .status()
            .expect("Git must be available in CI");
        assert!(git_status.success());

        let by_current_directory =
            resolve_unpin_target(None, &repository_directory.0, Platform::current())
                .expect("current repository must resolve");
        assert!(matches!(by_current_directory, UnpinTarget::Repository(_)));

        let by_existing_path = resolve_unpin_target(
            Some(repository_directory.0.as_os_str()),
            Path::new("unused"),
            Platform::current(),
        )
        .expect("existing repository path must resolve");
        assert!(matches!(by_existing_path, UnpinTarget::Repository(_)));

        let by_name = resolve_unpin_target(
            Some(std::ffi::OsStr::new("exact-project-name")),
            Path::new("unused"),
            Platform::current(),
        )
        .expect("missing path must be treated as an exact name");
        assert_eq!(by_name, UnpinTarget::Name("exact-project-name".to_owned()));
    }

    #[test]
    fn list_scans_mixed_records_sorts_them_and_does_not_require_code() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.join("launchers"));
        backend.fail_vscode.set(true);
        let valid_root = temporary.0.join("valid");
        initialize_repository(&valid_root);
        let valid = backend.launcher("zeta", &valid_root);
        let stale = backend.launcher("alpha", &temporary.0.join("deleted"));
        backend.set_enumeration(vec![
            Ok(valid.clone()),
            Err(LauncherEnumerationError::new(
                temporary.0.join("corrupt"),
                "corrupt fixture",
            )),
            Ok(stale.clone()),
        ]);

        let report = list(&backend, Platform::current()).unwrap();
        assert_eq!(
            report
                .records
                .iter()
                .map(|record| record.launcher.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        assert!(matches!(
            report.records[0].status,
            crate::repo::RootStatus::Invalid(_)
        ));
        assert_eq!(report.records[1].status, crate::repo::RootStatus::Valid);
        assert_eq!(report.errors.len(), 1);
        assert!(
            backend.removed.borrow().is_empty(),
            "list must have no side effects"
        );
    }

    #[test]
    fn prune_removes_all_stale_entries_is_idempotent_and_preserves_valid_or_foreign_entries() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.join("launchers"));
        let valid_root = temporary.0.join("valid");
        initialize_repository(&valid_root);
        let valid = backend.launcher("valid", &valid_root);
        let stale_one = backend.launcher("stale-one", &temporary.0.join("missing-one"));
        let stale_two = backend.launcher("stale-two", &temporary.0.join("missing-two"));
        backend.set_enumeration(vec![
            Ok(stale_two.clone()),
            Ok(valid),
            Ok(stale_one.clone()),
        ]);

        let report = prune(&backend, Platform::current()).unwrap();
        assert_eq!(
            report
                .removed
                .iter()
                .map(|launcher| launcher.name.as_str())
                .collect::<Vec<_>>(),
            ["stale-one", "stale-two"]
        );
        assert!(report.errors.is_empty());

        backend.set_enumeration(Vec::new());
        let repeat = prune(&backend, Platform::current()).unwrap();
        assert!(repeat.removed.is_empty());
        assert!(repeat.errors.is_empty());
    }

    #[test]
    fn prune_continues_after_enumeration_and_removal_failures() {
        let temporary = TempDir::new();
        let backend = FakeBackend::new(temporary.0.join("launchers"));
        let failed = backend.launcher("failed", &temporary.0.join("missing-failed"));
        let removed = backend.launcher("removed", &temporary.0.join("missing-removed"));
        backend
            .fail_remove_names
            .borrow_mut()
            .push("failed".to_owned());
        backend.set_enumeration(vec![
            Err(LauncherEnumerationError::new(
                temporary.0.join("broken-candidate"),
                "broken metadata",
            )),
            Ok(failed),
            Ok(removed.clone()),
        ]);

        let report = prune(&backend, Platform::current()).unwrap();
        assert_eq!(report.removed, vec![removed]);
        assert_eq!(report.errors.len(), 2);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("broken-candidate")));
        assert!(report.errors.iter().any(|error| error.contains("failed")));
    }
}
