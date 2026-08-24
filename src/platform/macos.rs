use std::env;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;
use crate::launcher::{
    CreateOutcome, LauncherBackend, LauncherInspection, LauncherRoot, ManagedLauncher,
};
use crate::repo::Repository;

const FORMAT_VERSION: &str = "1";
const ROOT_KEY: &str = "X-Git-Pin-Repository-Root";
const VERSION_KEY: &str = "X-Git-Pin-Format-Version";
const EXECUTABLE_NAME: &str = "git-pin-launcher";

/// macOS implementation backed by user-level application bundles.
pub struct MacOsBackend {
    root: LauncherRoot,
    launcher_binary: PathBuf,
}

impl MacOsBackend {
    /// Resolves the production user Applications root and current launcher binary.
    pub fn new() -> Result<Self, AppError> {
        let home = env::var_os("HOME").ok_or_else(|| {
            AppError::failure(
                "could not determine macOS launcher directory because HOME is not set",
            )
        })?;
        let home = PathBuf::from(home);
        if !home.is_absolute() {
            return Err(AppError::failure(format!(
                "macOS HOME '{}' must be an absolute path",
                home.display()
            )));
        }
        let current_executable = env::current_exe().map_err(|error| {
            AppError::failure(format!(
                "could not locate current git-pin executable for macOS bundles: {error}"
            ))
        })?;
        let launcher_binary = current_executable
            .parent()
            .ok_or_else(|| {
                AppError::failure(format!(
                    "current executable '{}' has no containing directory",
                    current_executable.display()
                ))
            })?
            .join(EXECUTABLE_NAME);
        Ok(Self {
            root: LauncherRoot::system(home.join("Applications/Git Pin")),
            launcher_binary,
        })
    }

    #[cfg(test)]
    fn for_test(root: PathBuf, launcher_binary: PathBuf) -> Self {
        Self {
            root: LauncherRoot::for_test(root),
            launcher_binary,
        }
    }

    fn bundle_path(&self, name: &str) -> PathBuf {
        self.root.as_path().join(format!("{name}.app"))
    }

    fn inspect_path(&self, name: &str, path: PathBuf) -> Result<LauncherInspection, AppError> {
        if !path.exists() {
            return Ok(LauncherInspection::Missing);
        }
        if !path.is_dir() {
            return Ok(LauncherInspection::Foreign { path });
        }

        let plist_path = path.join("Contents/Info.plist");
        let executable = path.join("Contents/MacOS").join(EXECUTABLE_NAME);
        let plist = match fs::read_to_string(&plist_path) {
            Ok(plist) => plist,
            Err(_) => return Ok(LauncherInspection::Foreign { path }),
        };
        let version = plist_string(&plist, VERSION_KEY);
        let root = plist_string(&plist, ROOT_KEY).map(PathBuf::from);
        let executable_is_valid = fs::metadata(&executable)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
        match (version.as_deref(), root, executable_is_valid) {
            (Some(FORMAT_VERSION), Some(root), true) if root.is_absolute() => {
                Ok(LauncherInspection::Managed(ManagedLauncher {
                    name: name.to_owned(),
                    root,
                    path,
                }))
            }
            _ => Ok(LauncherInspection::Foreign { path }),
        }
    }

    fn register(&self, bundle: &Path) -> Option<String> {
        let command = Path::new(
            "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
        );
        if !command.is_file() {
            return Some(format!(
                "Launch Services registration tool was not found; bundle '{}' remains valid",
                bundle.display()
            ));
        }
        match Command::new(command).arg("-f").arg(bundle).output() {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(format!(
                "Launch Services registration failed with status {}; bundle '{}' remains valid",
                output.status,
                bundle.display()
            )),
            Err(error) => Some(format!(
                "could not run Launch Services registration for '{}': {error}; bundle remains valid",
                bundle.display()
            )),
        }
    }
}

fn stable_bundle_identifier(root: &Path) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in root.as_os_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("io.github.git-pin.repository.{hash:016x}")
}

fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn info_plist(repository: &Repository) -> Result<String, AppError> {
    let root = repository.root().to_str().ok_or_else(|| {
        AppError::failure(format!(
            "repository root '{}' is not valid UTF-8 for macOS bundle metadata",
            repository.root().display()
        ))
    })?;
    let name = xml_escape(repository.name());
    let root = xml_escape(root);
    let identifier = stable_bundle_identifier(repository.root());

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>CFBundleDisplayName</key>\n  <string>{name}</string>\n  <key>CFBundleExecutable</key>\n  <string>{EXECUTABLE_NAME}</string>\n  <key>CFBundleIdentifier</key>\n  <string>{identifier}</string>\n  <key>CFBundleInfoDictionaryVersion</key>\n  <string>6.0</string>\n  <key>CFBundleName</key>\n  <string>{name}</string>\n  <key>CFBundlePackageType</key>\n  <string>APPL</string>\n  <key>{VERSION_KEY}</key>\n  <string>{FORMAT_VERSION}</string>\n  <key>{ROOT_KEY}</key>\n  <string>{root}</string>\n</dict>\n</plist>\n"
    ))
}

fn xml_unescape(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find('&') {
        output.push_str(&remaining[..index]);
        remaining = &remaining[index..];
        let (entity, replacement) = if remaining.starts_with("&amp;") {
            ("&amp;", '&')
        } else if remaining.starts_with("&lt;") {
            ("&lt;", '<')
        } else if remaining.starts_with("&gt;") {
            ("&gt;", '>')
        } else if remaining.starts_with("&quot;") {
            ("&quot;", '"')
        } else if remaining.starts_with("&apos;") {
            ("&apos;", '\'')
        } else {
            return None;
        };
        output.push(replacement);
        remaining = &remaining[entity.len()..];
    }
    output.push_str(remaining);
    Some(output)
}

fn plist_string(plist: &str, key: &str) -> Option<String> {
    let key_markup = format!("<key>{key}</key>");
    let after_key = plist.split_once(&key_markup)?.1.trim_start();
    let value = after_key
        .strip_prefix("<string>")?
        .split_once("</string>")?
        .0;
    xml_unescape(value)
}

fn unique_temporary_bundle(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    for sequence in 0..100_u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| AppError::failure(format!("system clock error: {error}")))?
            .as_nanos();
        let path = root.join(format!(
            ".{name}.{}-{nonce}-{sequence}.tmp.app",
            std::process::id()
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(AppError::failure(format!(
        "could not allocate a temporary macOS bundle path in '{}'",
        root.display()
    )))
}

impl LauncherBackend for MacOsBackend {
    fn launcher_root(&self) -> &LauncherRoot {
        &self.root
    }

    fn vscode_executable(&self) -> Result<PathBuf, AppError> {
        let application = PathBuf::from("/Applications/Visual Studio Code.app");
        application.is_dir().then_some(application).ok_or_else(|| {
            AppError::failure(
                "could not find stable Visual Studio Code at '/Applications/Visual Studio Code.app'",
            )
        })
    }

    fn inspect(&self, name: &str) -> Result<LauncherInspection, AppError> {
        let path = self.bundle_path(name);
        self.inspect_path(name, path)
    }

    fn create(&self, repository: &Repository, _vscode: &Path) -> Result<CreateOutcome, AppError> {
        let launcher_metadata = fs::metadata(&self.launcher_binary).map_err(|error| {
            AppError::failure(format!(
                "could not access current-architecture macOS launcher '{}': {error}",
                self.launcher_binary.display()
            ))
        })?;
        if !launcher_metadata.is_file() {
            return Err(AppError::failure(format!(
                "current-architecture macOS launcher '{}' is not a file",
                self.launcher_binary.display()
            )));
        }
        fs::create_dir_all(self.root.as_path()).map_err(|error| {
            AppError::failure(format!(
                "could not create macOS launcher directory '{}': {error}",
                self.root.as_path().display()
            ))
        })?;
        let final_path = self.bundle_path(repository.name());
        if final_path.exists() {
            return Ok(CreateOutcome::Occupied(
                self.inspect_path(repository.name(), final_path)?,
            ));
        }

        let temporary = unique_temporary_bundle(self.root.as_path(), repository.name())?;
        let prepare_result = (|| -> Result<(), AppError> {
            let contents = temporary.join("Contents");
            let macos = contents.join("MacOS");
            fs::create_dir_all(&macos).map_err(|error| {
                AppError::failure(format!(
                    "could not assemble temporary macOS bundle '{}': {error}",
                    temporary.display()
                ))
            })?;
            fs::write(contents.join("Info.plist"), info_plist(repository)?).map_err(|error| {
                AppError::failure(format!(
                    "could not write temporary macOS bundle metadata '{}': {error}",
                    temporary.display()
                ))
            })?;
            let installed_launcher = macos.join(EXECUTABLE_NAME);
            fs::copy(&self.launcher_binary, &installed_launcher).map_err(|error| {
                AppError::failure(format!(
                    "could not install current-architecture launcher in '{}': {error}",
                    temporary.display()
                ))
            })?;
            fs::set_permissions(&installed_launcher, fs::Permissions::from_mode(0o755)).map_err(
                |error| {
                    AppError::failure(format!(
                        "could not make bundle launcher '{}' executable: {error}",
                        installed_launcher.display()
                    ))
                },
            )?;
            match self.inspect_path(repository.name(), temporary.clone())? {
                LauncherInspection::Managed(launcher) if launcher.root == repository.root() => {
                    Ok(())
                }
                inspection => Err(AppError::failure(format!(
                    "temporary macOS bundle '{}' failed validation: {inspection:?}",
                    temporary.display()
                ))),
            }
        })();
        if let Err(error) = prepare_result {
            let _ = fs::remove_dir_all(&temporary);
            return Err(error);
        }

        match fs::rename(&temporary, &final_path) {
            Ok(()) => match self.inspect_path(repository.name(), final_path.clone())? {
                LauncherInspection::Managed(launcher) => {
                    if let Some(warning) = self.register(&final_path) {
                        eprintln!("warning: {warning}");
                    }
                    Ok(CreateOutcome::Created(launcher))
                }
                inspection => {
                    let _ = fs::remove_dir_all(&final_path);
                    Err(AppError::failure(format!(
                        "committed macOS bundle '{}' failed validation: {inspection:?}",
                        final_path.display()
                    )))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_dir_all(&temporary);
                Ok(CreateOutcome::Occupied(
                    self.inspect_path(repository.name(), final_path)?,
                ))
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&temporary);
                Err(AppError::failure(format!(
                    "could not atomically commit macOS bundle '{}': {error}",
                    final_path.display()
                )))
            }
        }
    }

    fn remove(&self, launcher: &ManagedLauncher) -> Result<(), AppError> {
        let expected_path = self.bundle_path(&launcher.name);
        if expected_path != launcher.path {
            return Err(AppError::failure(format!(
                "refusing to remove macOS bundle '{}' outside managed launcher root '{}'",
                launcher.path.display(),
                self.root.as_path().display()
            )));
        }
        match self.inspect_path(&launcher.name, launcher.path.clone())? {
            LauncherInspection::Missing => Ok(()),
            LauncherInspection::Managed(current) if current == *launcher => {
                fs::remove_dir_all(&current.path).map_err(|error| {
                    AppError::failure(format!(
                        "could not remove macOS bundle '{}': {error}",
                        current.path.display()
                    ))
                })
            }
            _ => Err(AppError::failure(format!(
                "refusing to remove macOS bundle '{}' because its managed metadata changed",
                launcher.path.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        info_plist, stable_bundle_identifier, MacOsBackend, EXECUTABLE_NAME, FORMAT_VERSION,
        ROOT_KEY, VERSION_KEY,
    };
    use crate::launcher::{CreateOutcome, LauncherBackend, LauncherInspection, ManagedLauncher};
    use crate::repo::{Platform, Repository};
    use crate::{
        app::{pin, unpin, PinOutcome, UnpinOutcome, UnpinTarget},
        macos_launcher::repository_root_for_launcher,
    };
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "git-pin-macos-backend-test-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temporary directory must be created");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn make_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").expect("executable fixture must be written");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("executable fixture permissions must be set");
    }

    fn test_backend(temporary: &TempDir) -> MacOsBackend {
        let launcher = temporary.0.join(EXECUTABLE_NAME);
        make_executable(&launcher);
        let vscode = temporary.0.join("Visual Studio Code.app");
        fs::create_dir_all(&vscode).expect("VS Code application fixture must be created");
        MacOsBackend::for_test(temporary.0.join("Applications"), launcher, vscode)
    }

    #[test]
    fn uses_user_applications_root_and_injected_test_root() {
        let backend = MacOsBackend::for_test(
            PathBuf::from("/tmp/applications"),
            PathBuf::from("/tmp/git-pin-launcher"),
            PathBuf::from("/tmp/Visual Studio Code.app"),
        );
        assert_eq!(
            backend.launcher_root().as_path(),
            Path::new("/tmp/applications")
        );
        assert_eq!(
            backend.bundle_path("project"),
            PathBuf::from("/tmp/applications/project.app")
        );
    }

    #[test]
    fn bundle_identifier_is_stable_and_distinguishes_roots() {
        let first = stable_bundle_identifier(Path::new("/work/project"));
        assert_eq!(first, stable_bundle_identifier(Path::new("/work/project")));
        assert_ne!(first, stable_bundle_identifier(Path::new("/other/project")));
        assert!(first.starts_with("io.github.git-pin.repository."));
    }

    #[test]
    fn serializes_controlled_plist_and_managed_metadata() {
        let repository = Repository::fixture(
            PathBuf::from("/work/项目 & <source> 'quoted'"),
            "项目 & <source>",
        );
        let plist = info_plist(&repository).expect("plist must serialize");

        assert!(plist.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(plist.contains("<string>项目 &amp; &lt;source&gt;</string>"));
        assert!(plist.contains("/work/项目 &amp; &lt;source&gt; &apos;quoted&apos;"));
        assert!(plist.contains(&format!("<key>{VERSION_KEY}</key>")));
        assert!(plist.contains(&format!("<string>{FORMAT_VERSION}</string>")));
        assert!(plist.contains(&format!("<key>{ROOT_KEY}</key>")));
        assert!(plist.contains(&format!("<string>{EXECUTABLE_NAME}</string>")));
        assert!(plist.contains(&stable_bundle_identifier(repository.root())));
    }

    #[test]
    fn assembles_inspects_and_safely_removes_an_application_bundle() {
        let temporary = TempDir::new();
        let backend = test_backend(&temporary);
        let repository = Repository::fixture(
            PathBuf::from("/work/项目 with spaces & shell;$HOME"),
            "项目 with spaces & shell;$HOME",
        );

        let managed = match backend
            .create(
                &repository,
                Path::new("/Applications/Visual Studio Code.app"),
            )
            .expect("bundle creation must succeed")
        {
            CreateOutcome::Created(launcher) => launcher,
            CreateOutcome::Occupied(_) => panic!("isolated bundle slot must be empty"),
        };
        let plist = managed.path.join("Contents/Info.plist");
        let installed_launcher = managed.path.join("Contents/MacOS").join(EXECUTABLE_NAME);
        assert!(plist.is_file());
        assert!(installed_launcher.is_file());
        assert_eq!(
            fs::metadata(&installed_launcher)
                .expect("installed launcher metadata must exist")
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("bundle inspection must succeed"),
            LauncherInspection::Managed(managed.clone())
        );
        assert!(!backend
            .launcher_root()
            .as_path()
            .read_dir()
            .expect("Applications root must be readable")
            .any(|entry| entry
                .expect("directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp.app")));

        backend
            .remove(&managed)
            .expect("managed bundle must be removed");
        assert!(!managed.path.exists());
    }

    #[test]
    fn preserves_foreign_and_out_of_root_bundles() {
        let temporary = TempDir::new();
        let applications = temporary.0.join("Applications");
        fs::create_dir_all(&applications).expect("Applications root must be created");
        let backend = test_backend(&temporary);
        let foreign = applications.join("project.app");
        fs::create_dir_all(&foreign).expect("foreign bundle must be created");
        fs::write(foreign.join("foreign"), "preserve").expect("foreign marker must be created");
        let repository = Repository::fixture(PathBuf::from("/work/project"), "project");

        assert_eq!(
            backend
                .create(
                    &repository,
                    Path::new("/Applications/Visual Studio Code.app")
                )
                .expect("occupied slot must be reported"),
            CreateOutcome::Occupied(LauncherInspection::Foreign {
                path: foreign.clone()
            })
        );
        assert!(foreign.join("foreign").is_file());

        let outside = ManagedLauncher {
            name: "project".to_owned(),
            root: PathBuf::from("/work/project"),
            path: temporary.0.join("outside.app"),
        };
        assert!(backend.remove(&outside).is_err());
    }

    #[test]
    fn launch_services_registration_is_best_effort() {
        let temporary = TempDir::new();
        let failing_registration = temporary.0.join("lsregister");
        fs::write(&failing_registration, "#!/bin/sh\nexit 9\n")
            .expect("registration fixture must be written");
        fs::set_permissions(&failing_registration, fs::Permissions::from_mode(0o755))
            .expect("registration fixture must be executable");
        let backend = test_backend(&temporary).with_registration_command(failing_registration);
        let repository = Repository::fixture(PathBuf::from("/work/project"), "project");

        let outcome = backend
            .create(
                &repository,
                Path::new("/Applications/Visual Studio Code.app"),
            )
            .expect("registration failure must not fail bundle creation");
        assert!(matches!(outcome, CreateOutcome::Created(_)));
        assert!(backend.bundle_path("project").is_dir());
    }

    #[test]
    fn isolated_bundle_backend_integrates_with_launcher_and_shared_orchestration() {
        let temporary = TempDir::new();
        let backend = test_backend(&temporary);
        let repository = Repository::fixture(
            PathBuf::from("/work/项目 with spaces & shell;$HOME`literal`"),
            "项目 with spaces & shell;$HOME`literal`",
        );

        let launcher = match pin(&backend, &repository, Platform::MacOs)
            .expect("first pin must create an application bundle")
        {
            PinOutcome::Created(launcher) => launcher,
            PinOutcome::AlreadyPinned(_) => panic!("isolated bundle slot must initially be empty"),
        };
        let embedded_launcher = launcher.path.join("Contents/MacOS").join(EXECUTABLE_NAME);
        assert_eq!(
            repository_root_for_launcher(&embedded_launcher)
                .expect("internal launcher must read the exact repository root"),
            repository.root()
        );
        assert!(matches!(
            pin(&backend, &repository, Platform::MacOs).expect("repeat pin must be idempotent"),
            PinOutcome::AlreadyPinned(_)
        ));

        let conflicting = Repository::fixture(
            PathBuf::from("/different/项目 with spaces & shell;$HOME`literal`"),
            repository.name(),
        );
        let error = pin(&backend, &conflicting, Platform::MacOs)
            .expect_err("same-name different-root pin must conflict");
        assert!(error.to_string().contains("already points to"));

        assert!(matches!(
            unpin(
                &backend,
                &UnpinTarget::Repository(repository.clone()),
                Platform::MacOs
            )
            .expect("matching repository must unpin"),
            UnpinOutcome::Removed(_)
        ));
        assert!(!launcher.path.exists());
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("removed bundle must inspect cleanly"),
            LauncherInspection::Missing
        );
    }
}
