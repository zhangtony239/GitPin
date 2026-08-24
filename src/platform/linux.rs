use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;
use crate::launcher::{
    CreateOutcome, LauncherBackend, LauncherInspection, LauncherRoot, ManagedLauncher,
};
use crate::repo::Repository;

const FILE_PREFIX: &str = "git-pin-";
const FILE_SUFFIX: &str = ".desktop";
const FORMAT_VERSION: &str = "1";
const ROOT_KEY: &str = "X-Git-Pin-Repository-Root";
const VERSION_KEY: &str = "X-Git-Pin-Format-Version";

/// Linux implementation backed by XDG Desktop Entry files.
pub struct LinuxBackend {
    root: LauncherRoot,
    path: Option<OsString>,
    vscode_override: Option<PathBuf>,
    refresh_override: Option<PathBuf>,
}

impl LinuxBackend {
    /// Resolves the production XDG applications directory from the environment.
    pub fn new() -> Result<Self, AppError> {
        let root = launcher_root_from_environment(
            env::var_os("XDG_DATA_HOME").as_deref(),
            env::var_os("HOME").as_deref(),
        )?;
        Ok(Self {
            root: LauncherRoot::system(root),
            path: env::var_os("PATH"),
            vscode_override: None,
            refresh_override: None,
        })
    }

    #[cfg(test)]
    fn for_test(root: PathBuf, vscode: PathBuf) -> Self {
        Self {
            root: LauncherRoot::for_test(root),
            path: None,
            vscode_override: Some(vscode),
            refresh_override: Some(PathBuf::from(
                "git-pin-disabled-update-desktop-database",
            )),
        }
    }

    #[cfg(test)]
    fn with_refresh_command(mut self, command: PathBuf) -> Self {
        self.refresh_override = Some(command);
        self
    }

    fn launcher_path(&self, name: &str) -> PathBuf {
        self.root
            .as_path()
            .join(format!("{FILE_PREFIX}{name}{FILE_SUFFIX}"))
    }

    fn inspect_path(&self, name: &str, path: PathBuf) -> Result<LauncherInspection, AppError> {
        if !path.exists() {
            return Ok(LauncherInspection::Missing);
        }
        if !path.is_file() {
            return Ok(LauncherInspection::Foreign { path });
        }

        let content = fs::read_to_string(&path).map_err(|error| {
            AppError::failure(format!(
                "could not read Linux launcher '{}': {error}",
                path.display()
            ))
        })?;
        match parse_managed_root(&content) {
            Some(root) => Ok(LauncherInspection::Managed(ManagedLauncher {
                name: name.to_owned(),
                root,
                path,
            })),
            None => Ok(LauncherInspection::Foreign { path }),
        }
    }

    fn refresh(&self) -> Option<String> {
        let command = match &self.refresh_override {
            Some(command) => command.clone(),
            None => match find_command("update-desktop-database", self.path.as_deref()) {
                Some(command) => command,
                None => {
                    return Some(format!(
                        "update-desktop-database was not found; launcher '{}' remains valid",
                        self.root.as_path().display()
                    ));
                }
            },
        };

        match Command::new(&command).arg(self.root.as_path()).output() {
            Ok(output) if output.status.success() => None,
            Ok(output) => Some(format!(
                "desktop database refresh command '{}' failed with status {}; launcher files remain valid",
                command.display(),
                output.status
            )),
            Err(error) => Some(format!(
                "could not run desktop database refresh command '{}': {error}; launcher files remain valid",
                command.display()
            )),
        }
    }
}

fn launcher_root_from_environment(
    xdg_data_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf, AppError> {
    let data_home = match xdg_data_home.filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => PathBuf::from(home.ok_or_else(|| {
            AppError::failure(
                "could not determine Linux launcher directory: neither XDG_DATA_HOME nor HOME is set",
            )
        })?)
        .join(".local/share"),
    };

    if !data_home.is_absolute() {
        return Err(AppError::failure(format!(
            "Linux data home '{}' must be an absolute path",
            data_home.display()
        )));
    }
    Ok(data_home.join("applications"))
}

fn find_vscode(path: Option<&OsStr>) -> Result<PathBuf, AppError> {
    let mut candidates = Vec::new();
    if let Some(path) = path {
        candidates.extend(env::split_paths(path).map(|directory| directory.join("code")));
    }
    candidates.extend([
        PathBuf::from("/usr/bin/code"),
        PathBuf::from("/usr/local/bin/code"),
        PathBuf::from("/snap/bin/code"),
    ]);

    candidates
        .into_iter()
        .find(|candidate| is_executable_file(candidate))
        .and_then(|candidate| fs::canonicalize(candidate).ok())
        .ok_or_else(|| {
            AppError::failure(
                "could not find the stable Visual Studio Code 'code' executable in PATH or a standard installation location",
            )
        })
}

fn find_command(name: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    path.into_iter()
        .flat_map(env::split_paths)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn desktop_entry(repository: &Repository, vscode: &Path) -> Result<String, AppError> {
    let vscode = vscode.to_str().ok_or_else(|| {
        AppError::failure(format!(
            "Visual Studio Code path '{}' is not valid UTF-8",
            vscode.display()
        ))
    })?;
    let root = repository.root().to_str().ok_or_else(|| {
        AppError::failure(format!(
            "repository root '{}' is not valid UTF-8",
            repository.root().display()
        ))
    })?;

    Ok(format!(
        "[Desktop Entry]\nType=Application\nName={}\nIcon=visual-studio-code\nTerminal=false\nExec={} {}\n{VERSION_KEY}={FORMAT_VERSION}\n{ROOT_KEY}={}\n",
        escape_value(repository.name()),
        quote_exec_argument(vscode),
        quote_exec_argument(root),
        escape_value(root)
    ))
}

fn escape_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn unescape_value(value: &str) -> Option<String> {
    let mut unescaped = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            unescaped.push(character);
            continue;
        }
        match characters.next()? {
            '\\' => unescaped.push('\\'),
            'n' => unescaped.push('\n'),
            'r' => unescaped.push('\r'),
            't' => unescaped.push('\t'),
            _ => return None,
        }
    }
    Some(unescaped)
}

fn quote_exec_argument(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');
    for character in argument.chars() {
        if matches!(character, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

fn parse_managed_root(content: &str) -> Option<PathBuf> {
    let mut in_desktop_entry = false;
    let mut version = None;
    let mut root = None;

    for line in content.lines() {
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(value) = line.strip_prefix(&format!("{VERSION_KEY}=")) {
            version = Some(value);
        } else if let Some(value) = line.strip_prefix(&format!("{ROOT_KEY}=")) {
            root = unescape_value(value).map(PathBuf::from);
        }
    }

    match (version, root) {
        (Some(FORMAT_VERSION), Some(root)) if root.is_absolute() => Some(root),
        _ => None,
    }
}

fn unique_temporary_path(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    for sequence in 0..100_u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| AppError::failure(format!("system clock error: {error}")))?
            .as_nanos();
        let path = root.join(format!(
            ".{FILE_PREFIX}{name}.{}-{nonce}-{sequence}.tmp",
            std::process::id()
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(AppError::failure(format!(
        "could not allocate a temporary launcher path in '{}'",
        root.display()
    )))
}

impl LauncherBackend for LinuxBackend {
    fn launcher_root(&self) -> &LauncherRoot {
        &self.root
    }

    fn vscode_executable(&self) -> Result<PathBuf, AppError> {
        match &self.vscode_override {
            Some(vscode) if is_executable_file(vscode) => Ok(vscode.clone()),
            Some(vscode) => Err(AppError::failure(format!(
                "injected Visual Studio Code executable '{}' is not executable",
                vscode.display()
            ))),
            None => find_vscode(self.path.as_deref()),
        }
    }

    fn inspect(&self, name: &str) -> Result<LauncherInspection, AppError> {
        let path = self.launcher_path(name);
        self.inspect_path(name, path)
    }

    fn create(
        &self,
        repository: &Repository,
        vscode: &Path,
    ) -> Result<CreateOutcome, AppError> {
        fs::create_dir_all(self.root.as_path()).map_err(|error| {
            AppError::failure(format!(
                "could not create Linux launcher directory '{}': {error}",
                self.root.as_path().display()
            ))
        })?;

        let content = desktop_entry(repository, vscode)?;
        let temporary = unique_temporary_path(self.root.as_path(), repository.name())?;
        let final_path = self.launcher_path(repository.name());
        let write_result = (|| -> Result<(), AppError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| {
                    AppError::failure(format!(
                        "could not create temporary Linux launcher '{}': {error}",
                        temporary.display()
                    ))
                })?;
            file.write_all(content.as_bytes()).map_err(|error| {
                AppError::failure(format!(
                    "could not write temporary Linux launcher '{}': {error}",
                    temporary.display()
                ))
            })?;
            file.sync_all().map_err(|error| {
                AppError::failure(format!(
                    "could not flush temporary Linux launcher '{}': {error}",
                    temporary.display()
                ))
            })?;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755)).map_err(
                |error| {
                    AppError::failure(format!(
                        "could not make temporary Linux launcher '{}' executable: {error}",
                        temporary.display()
                    ))
                },
            )?;
            match self.inspect_path(repository.name(), temporary.clone())? {
                LauncherInspection::Managed(launcher)
                    if launcher.root == repository.root() => Ok(()),
                _ => Err(AppError::failure(format!(
                    "temporary Linux launcher '{}' failed managed metadata validation",
                    temporary.display()
                ))),
            }
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }

        match fs::hard_link(&temporary, &final_path) {
            Ok(()) => {
                let _ = fs::remove_file(&temporary);
                match self.inspect(repository.name())? {
                    LauncherInspection::Managed(launcher) => {
                        if let Some(warning) = self.refresh() {
                            eprintln!("warning: {warning}");
                        }
                        Ok(CreateOutcome::Created(launcher))
                    }
                    _ => Err(AppError::failure(format!(
                        "committed Linux launcher '{}' failed validation",
                        final_path.display()
                    ))),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temporary);
                Ok(CreateOutcome::Occupied(self.inspect(repository.name())?))
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(AppError::failure(format!(
                    "could not atomically commit Linux launcher '{}': {error}",
                    final_path.display()
                )))
            }
        }
    }

    fn remove(&self, launcher: &ManagedLauncher) -> Result<(), AppError> {
        match self.inspect(&launcher.name)? {
            LauncherInspection::Missing => Ok(()),
            LauncherInspection::Managed(current) if current == *launcher => {
                fs::remove_file(&current.path).map_err(|error| {
                    AppError::failure(format!(
                        "could not remove Linux launcher '{}': {error}",
                        current.path.display()
                    ))
                })?;
                if let Some(warning) = self.refresh() {
                    eprintln!("warning: {warning}");
                }
                Ok(())
            }
            _ => Err(AppError::failure(format!(
                "refusing to remove Linux launcher '{}' because its managed metadata changed",
                launcher.path.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        desktop_entry, find_vscode, launcher_root_from_environment, LinuxBackend, FILE_PREFIX,
        FORMAT_VERSION, ROOT_KEY, VERSION_KEY,
    };
    use crate::launcher::{CreateOutcome, LauncherBackend, LauncherInspection};
    use crate::repo::{Platform, Repository};
    use crate::{
        app::{pin, unpin, PinOutcome, UnpinOutcome, UnpinTarget},
        launcher::ManagedLauncher,
    };
    use std::ffi::OsStr;
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
                "git-pin-linux-test-{}-{nonce}",
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

    #[test]
    fn resolves_xdg_data_home_or_home_fallback() {
        assert_eq!(
            launcher_root_from_environment(Some(OsStr::new("/data")), None)
                .expect("absolute XDG_DATA_HOME must work"),
            PathBuf::from("/data/applications")
        );
        assert_eq!(
            launcher_root_from_environment(None, Some(OsStr::new("/home/user")))
                .expect("HOME fallback must work"),
            PathBuf::from("/home/user/.local/share/applications")
        );
        assert!(launcher_root_from_environment(Some(OsStr::new("relative")), None).is_err());
        assert!(launcher_root_from_environment(None, None).is_err());
    }

    #[test]
    fn finds_an_executable_code_from_path() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        assert_eq!(
            find_vscode(Some(temporary.0.as_os_str())).expect("Code must be found"),
            fs::canonicalize(code).expect("fixture must canonicalize")
        );
    }

    #[test]
    fn uses_prefixed_launcher_names_and_managed_metadata_keys() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        let backend = LinuxBackend::for_test(temporary.0.clone(), code);
        assert_eq!(
            backend.launcher_path("project"),
            temporary.0.join("git-pin-project.desktop")
        );
        assert_eq!(FILE_PREFIX, "git-pin-");
        assert_eq!(ROOT_KEY, "X-Git-Pin-Repository-Root");
        assert_eq!(VERSION_KEY, "X-Git-Pin-Format-Version");
        assert_eq!(FORMAT_VERSION, "1");
        assert!(backend.vscode_executable().is_ok());
    }

    #[test]
    fn desktop_entry_encodes_fields_and_exec_arguments_without_a_shell() {
        let repository = Repository::fixture(
            PathBuf::from("/work/项目 $HOME; `echo no` \\ path"),
            "项目 with spaces",
        );
        let entry = desktop_entry(&repository, Path::new("/opt/Visual Studio Code/code"))
            .expect("Desktop Entry must serialize");

        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("Type=Application\n"));
        assert!(entry.contains("Name=项目 with spaces\n"));
        assert!(entry.contains("Icon=visual-studio-code\n"));
        assert!(entry.contains("Terminal=false\n"));
        assert!(entry.contains(
            "Exec=\"/opt/Visual Studio Code/code\" \"/work/项目 \\$HOME; \\`echo no\\` \\\\ path\"\n"
        ));
        assert!(!entry.contains("sh -c"));
        assert!(entry.contains(&format!("{VERSION_KEY}={FORMAT_VERSION}\n")));
        assert!(entry.contains(&format!(
            "{ROOT_KEY}=/work/项目 $HOME; `echo no` \\\\ path\n"
        )));
    }

    #[test]
    fn creates_inspects_and_safely_removes_a_managed_entry() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        let backend = LinuxBackend::for_test(temporary.0.join("applications"), code.clone());
        let repository = Repository::fixture(
            temporary.0.join("项目 with spaces;$HOME"),
            "项目 with spaces;$HOME",
        );

        let launcher = match backend
            .create(&repository, &code)
            .expect("launcher creation must succeed")
        {
            CreateOutcome::Created(launcher) => launcher,
            CreateOutcome::Occupied(_) => panic!("new isolated slot must not be occupied"),
        };
        let metadata = fs::metadata(&launcher.path).expect("launcher must exist");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("inspection must succeed"),
            LauncherInspection::Managed(launcher.clone())
        );
        assert!(!backend
            .launcher_root()
            .as_path()
            .read_dir()
            .expect("launcher root must be readable")
            .any(|entry| entry
                .expect("directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));

        backend.remove(&launcher).expect("removal must succeed");
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("inspection must succeed"),
            LauncherInspection::Missing
        );
    }

    #[test]
    fn preserves_existing_and_foreign_launcher_slots() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        let applications = temporary.0.join("applications");
        fs::create_dir_all(&applications).expect("applications directory must be created");
        let backend = LinuxBackend::for_test(applications.clone(), code.clone());
        let repository = Repository::fixture(PathBuf::from("/work/project"), "project");
        let foreign_path = applications.join("git-pin-project.desktop");
        fs::write(&foreign_path, "[Desktop Entry]\nType=Application\n")
            .expect("foreign fixture must be written");

        let outcome = backend
            .create(&repository, &code)
            .expect("occupied create must be reported");
        assert_eq!(
            outcome,
            CreateOutcome::Occupied(LauncherInspection::Foreign {
                path: foreign_path.clone()
            })
        );
        assert_eq!(
            fs::read_to_string(&foreign_path).expect("foreign fixture must remain"),
            "[Desktop Entry]\nType=Application\n"
        );
    }

    #[test]
    fn desktop_database_refresh_is_best_effort() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);

        let success = temporary.0.join("refresh-success");
        fs::write(&success, "#!/bin/sh\nexit 0\n").expect("refresh fixture must be written");
        fs::set_permissions(&success, fs::Permissions::from_mode(0o755))
            .expect("refresh fixture permissions must be set");
        let successful_backend = LinuxBackend::for_test(
            temporary.0.join("success-applications"),
            code.clone(),
        )
        .with_refresh_command(success);
        assert_eq!(successful_backend.refresh(), None);

        let failure = temporary.0.join("refresh-failure");
        fs::write(&failure, "#!/bin/sh\nexit 7\n").expect("refresh fixture must be written");
        fs::set_permissions(&failure, fs::Permissions::from_mode(0o755))
            .expect("refresh fixture permissions must be set");
        let failing_backend =
            LinuxBackend::for_test(temporary.0.join("failure-applications"), code.clone())
                .with_refresh_command(failure);
        let warning = failing_backend
            .refresh()
            .expect("non-zero refresh must produce a warning");
        assert!(warning.contains("status"));
        assert!(warning.contains("launcher files remain valid"));

        let missing_backend =
            LinuxBackend::for_test(temporary.0.join("missing-applications"), code)
                .with_refresh_command(temporary.0.join("missing-refresh-tool"));
        let warning = missing_backend
            .refresh()
            .expect("missing refresh command must produce a warning");
        assert!(warning.contains("could not run desktop database refresh"));
        assert!(warning.contains("launcher files remain valid"));
    }

    #[test]
    fn failed_refresh_does_not_roll_back_a_valid_launcher() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        let backend = LinuxBackend::for_test(temporary.0.join("applications"), code.clone())
            .with_refresh_command(temporary.0.join("missing-refresh-tool"));
        let repository = Repository::fixture(PathBuf::from("/work/project"), "project");

        let outcome = backend
            .create(&repository, &code)
            .expect("refresh failure must not fail creation");
        let launcher = match outcome {
            CreateOutcome::Created(launcher) => launcher,
            CreateOutcome::Occupied(_) => panic!("isolated slot must not be occupied"),
        };
        assert!(launcher.path.is_file());
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("launcher must remain inspectable"),
            LauncherInspection::Managed(launcher)
        );
    }

    #[test]
    fn isolated_backend_integrates_with_pin_conflict_and_unpin_orchestration() {
        let temporary = TempDir::new();
        let code = temporary.0.join("code");
        make_executable(&code);
        let backend = LinuxBackend::for_test(temporary.0.join("applications"), code);
        let repository = Repository::fixture(
            PathBuf::from("/work/项目 with spaces;$HOME`literal`"),
            "项目 with spaces;$HOME`literal`",
        );

        let launcher = match pin(&backend, &repository, Platform::Linux)
            .expect("first pin must create the Desktop Entry")
        {
            PinOutcome::Created(launcher) => launcher,
            PinOutcome::AlreadyPinned(_) => panic!("isolated slot must initially be empty"),
        };
        assert!(matches!(
            backend
                .inspect(repository.name())
                .expect("managed launcher must be readable"),
            LauncherInspection::Managed(ManagedLauncher { root, .. }) if root == repository.root()
        ));
        assert!(matches!(
            pin(&backend, &repository, Platform::Linux).expect("repeat pin must be idempotent"),
            PinOutcome::AlreadyPinned(_)
        ));

        let conflicting = Repository::fixture(
            PathBuf::from("/different/项目 with spaces;$HOME`literal`"),
            repository.name(),
        );
        let error = pin(&backend, &conflicting, Platform::Linux)
            .expect_err("same-name different-root pin must conflict");
        assert!(error.to_string().contains(&launcher.root.display().to_string()));

        assert!(matches!(
            unpin(
                &backend,
                &UnpinTarget::Repository(repository.clone()),
                Platform::Linux
            )
            .expect("matching repository must unpin"),
            UnpinOutcome::Removed(_)
        ));
        assert!(!launcher.path.exists());
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("removed slot must inspect cleanly"),
            LauncherInspection::Missing
        );
    }
}
