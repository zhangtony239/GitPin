//! Git-backed configuration and IDE executable resolution.

use std::env;
use std::ffi::OsStr;
#[cfg(windows)]
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;
use crate::repo::Platform;

pub const IDE_CONFIG_KEY: &str = "pin.ide";
pub const DEFAULT_IDE: &str = "code";

/// The configured IDE value together with whether Git supplied it explicitly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdeSelection {
    DefaultCode,
    Configured(String),
}

impl IdeSelection {
    pub fn value(&self) -> &str {
        match self {
            Self::DefaultCode => DEFAULT_IDE,
            Self::Configured(value) => value,
        }
    }
}

/// Resolves an IDE selection while preserving the Windows default-Code policy.
pub fn resolve_ide_selection(
    selection: &IdeSelection,
    platform: Platform,
) -> Result<PathBuf, AppError> {
    let path = env::var_os("PATH");
    resolve_ide_selection_with_path(selection, platform, path.as_deref())
}

pub(crate) fn resolve_ide_selection_with_path(
    selection: &IdeSelection,
    platform: Platform,
    path: Option<&OsStr>,
) -> Result<PathBuf, AppError> {
    match (selection, platform) {
        #[cfg(windows)]
        (IdeSelection::DefaultCode, Platform::Windows) => resolve_windows_default_code(path),
        _ => resolve_ide_with_path(selection.value(), path),
    }
}

/// Reads the effective IDE value using Git's own configuration machinery.
pub fn read_ide() -> Result<IdeSelection, AppError> {
    let mut command = Command::new("git");
    read_ide_with(&mut command)
}

fn read_ide_with(command: &mut Command) -> Result<IdeSelection, AppError> {
    let output = command
        .args(["config", "--get", IDE_CONFIG_KEY])
        .output()
        .map_err(|error| {
            AppError::failure(format!(
                "could not read Git configuration key '{IDE_CONFIG_KEY}': {error}"
            ))
        })?;

    if output.status.success() {
        let value = String::from_utf8(output.stdout).map_err(|error| {
            AppError::failure(format!(
                "Git configuration key '{IDE_CONFIG_KEY}' is not valid UTF-8: {error}"
            ))
        })?;
        return Ok(IdeSelection::Configured(
            value.trim_end_matches(['\r', '\n']).to_owned(),
        ));
    }

    if output.status.code() == Some(1) && output.stderr.is_empty() {
        return Ok(IdeSelection::DefaultCode);
    }

    let diagnostic = String::from_utf8_lossy(&output.stderr);
    Err(AppError::failure(format!(
        "could not read Git configuration key '{IDE_CONFIG_KEY}' (status {}): {}",
        output.status,
        diagnostic.trim()
    )))
}

/// Resolves one atomic configured value to an absolute executable file.
pub fn resolve_ide(value: &str) -> Result<PathBuf, AppError> {
    let path = env::var_os("PATH");
    resolve_ide_with_path(value, path.as_deref())
}

fn resolve_ide_with_path(value: &str, path: Option<&OsStr>) -> Result<PathBuf, AppError> {
    if value.is_empty() {
        return Err(invalid_ide(value, "the value is empty"));
    }

    let configured = Path::new(value);
    let candidate = if has_directory_component(configured) {
        if configured.is_absolute() {
            configured.to_owned()
        } else {
            env::current_dir()
                .map_err(|error| {
                    AppError::failure(format!(
                        "could not resolve Git configuration key '{IDE_CONFIG_KEY}' value {value:?}: could not determine current directory: {error}"
                    ))
                })?
                .join(configured)
        }
    } else {
        find_on_path(configured.as_os_str(), path).ok_or_else(|| {
            invalid_ide(
                value,
                "no executable with that exact name was found in PATH; values are not split into a command and arguments",
            )
        })?
    };

    validate_and_absolute(value, &candidate)
}

#[cfg(windows)]
fn resolve_windows_default_code(path: Option<&OsStr>) -> Result<PathBuf, AppError> {
    let selected = find_on_path(OsStr::new(DEFAULT_IDE), path).ok_or_else(|| {
        default_code_error("no CLI shim or GUI executable named 'code' was found in PATH")
    })?;
    let extension = selected
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    if extension.eq_ignore_ascii_case("exe") {
        return validate_windows_default_gui(&selected, &selected);
    }

    if extension.eq_ignore_ascii_case("cmd") {
        let install_directory = selected.parent().and_then(Path::parent);
        let candidate = install_directory
            .map(|directory| directory.join("Code.exe"))
            .unwrap_or_else(|| PathBuf::from("Code.exe"));
        let is_standard_bin = selected
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .map(|name| name.eq_ignore_ascii_case("bin"))
            .unwrap_or(false);
        if !is_standard_bin {
            return Err(default_code_error(format!(
                "PATH selected script shim '{}', but it is not in the supported 'bin' layout; GUI candidate would be '{}'",
                selected.display(),
                candidate.display()
            )));
        }
        return validate_windows_default_gui(&selected, &candidate);
    }

    Err(default_code_error(format!(
        "PATH selected unsupported executable '{}'; expected a direct .exe or a standard bin\\code.cmd shim, and script fallback is disabled",
        selected.display()
    )))
}

#[cfg(windows)]
fn validate_windows_default_gui(shim: &Path, candidate: &Path) -> Result<PathBuf, AppError> {
    let extension_is_exe = candidate
        .extension()
        .and_then(OsStr::to_str)
        .map(|extension| extension.eq_ignore_ascii_case("exe"))
        .unwrap_or(false);
    if !extension_is_exe {
        return Err(default_code_error(format!(
            "PATH selection '{}' produced GUI candidate '{}', which is not an .exe",
            shim.display(),
            candidate.display()
        )));
    }

    let metadata = fs::metadata(candidate).map_err(|error| {
        default_code_error(format!(
            "PATH selection '{}' produced unavailable GUI candidate '{}': {error}",
            shim.display(),
            candidate.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(default_code_error(format!(
            "PATH selection '{}' produced GUI candidate '{}', which is not a file",
            shim.display(),
            candidate.display()
        )));
    }
    let canonical = fs::canonicalize(candidate).map_err(|error| {
        default_code_error(format!(
            "could not make GUI candidate '{}' absolute for PATH selection '{}': {error}",
            candidate.display(),
            shim.display()
        ))
    })?;
    Ok(shell_compatible_absolute(canonical))
}

#[cfg(windows)]
fn default_code_error(reason: impl std::fmt::Display) -> AppError {
    AppError::failure(format!(
        "could not resolve Windows GUI executable for default IDE '{DEFAULT_IDE}': {reason}"
    ))
}

fn has_directory_component(path: &Path) -> bool {
    path.is_absolute() || path.components().count() > 1
}

fn validate_and_absolute(value: &str, candidate: &Path) -> Result<PathBuf, AppError> {
    let metadata = fs::metadata(candidate).map_err(|error| {
        invalid_ide(
            value,
            format!("target '{}' is unavailable: {error}", candidate.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(invalid_ide(
            value,
            format!("target '{}' is not a file", candidate.display()),
        ));
    }
    if !is_executable(candidate, &metadata) {
        return Err(invalid_ide(
            value,
            format!("target '{}' is not executable", candidate.display()),
        ));
    }
    let canonical = fs::canonicalize(candidate).map_err(|error| {
        invalid_ide(
            value,
            format!(
                "could not make target '{}' absolute: {error}",
                candidate.display()
            ),
        )
    })?;
    Ok(shell_compatible_absolute(canonical))
}

#[cfg(windows)]
fn shell_compatible_absolute(path: PathBuf) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if let Some(local) = text.strip_prefix(r"\\?\") {
        if let Some(unc) = local.strip_prefix("UNC\\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        return PathBuf::from(local);
    }
    path
}

#[cfg(not(windows))]
fn shell_compatible_absolute(path: PathBuf) -> PathBuf {
    path
}

fn find_on_path(name: &OsStr, path: Option<&OsStr>) -> Option<PathBuf> {
    let path = path?;
    for directory in env::split_paths(&path) {
        for candidate in executable_candidates(&directory, name) {
            if fs::metadata(&candidate)
                .map(|metadata| metadata.is_file() && is_executable(&candidate, &metadata))
                .unwrap_or(false)
            {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
fn executable_candidates(directory: &Path, name: &OsStr) -> Vec<PathBuf> {
    let direct = directory.join(name);
    if direct.extension().is_some() {
        return vec![direct];
    }
    let extensions =
        env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
    extensions
        .to_string_lossy()
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| {
            let mut file_name = name.to_os_string();
            file_name.push(extension);
            directory.join(file_name)
        })
        .collect()
}

#[cfg(not(windows))]
fn executable_candidates(directory: &Path, name: &OsStr) -> Vec<PathBuf> {
    vec![directory.join(name)]
}

#[cfg(unix)]
fn is_executable(_path: &Path, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(path: &Path, _metadata: &fs::Metadata) -> bool {
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    env::var_os("PATHEXT")
        .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"))
        .to_string_lossy()
        .split(';')
        .any(|allowed| {
            allowed
                .trim_start_matches('.')
                .eq_ignore_ascii_case(extension)
        })
}

fn invalid_ide(value: &str, reason: impl std::fmt::Display) -> AppError {
    AppError::failure(format!(
        "Git configuration key '{IDE_CONFIG_KEY}' has invalid executable value {value:?}: {reason}"
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        read_ide_with, resolve_ide, shell_compatible_absolute, IdeSelection, IDE_CONFIG_KEY,
    };
    #[cfg(windows)]
    use super::resolve_ide_selection_with_path;
    #[cfg(windows)]
    use crate::repo::Platform;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
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
            let path = env::temp_dir().join(format!(
                "git-pin-config-test-{}-{nonce}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
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

    fn git(directory: &Path, arguments: &[&str]) {
        assert!(Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(arguments)
            .status()
            .expect("Git must run")
            .success());
    }

    fn isolated_git<'a>(directory: &'a Path, temporary: &'a TempDir) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(directory)
            .env("GIT_CONFIG_SYSTEM", temporary.0.join("system.gitconfig"))
            .env("GIT_CONFIG_GLOBAL", temporary.0.join("global.gitconfig"));
        command
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    fn make_executable(path: &Path) {
        fs::copy(env::current_exe().unwrap(), path).unwrap();
    }

    #[test]
    fn defaults_and_obeys_git_scope_and_command_line_precedence() {
        let temporary = TempDir::new();
        let repository = temporary.0.join("repository");
        fs::create_dir_all(&repository).unwrap();
        git(&repository, &["init"]);

        assert_eq!(
            read_ide_with(&mut isolated_git(&repository, &temporary)).unwrap(),
            IdeSelection::DefaultCode
        );

        git(
            &repository,
            &[
                "config",
                "--file",
                temporary.0.join("system.gitconfig").to_str().unwrap(),
                IDE_CONFIG_KEY,
                "system-ide",
            ],
        );
        git(
            &repository,
            &[
                "config",
                "--file",
                temporary.0.join("global.gitconfig").to_str().unwrap(),
                IDE_CONFIG_KEY,
                "global-ide",
            ],
        );
        git(&repository, &["config", IDE_CONFIG_KEY, "repository-ide"]);
        assert_eq!(
            read_ide_with(&mut isolated_git(&repository, &temporary)).unwrap(),
            IdeSelection::Configured("repository-ide".to_owned())
        );

        let mut command_line = isolated_git(&repository, &temporary);
        command_line
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", IDE_CONFIG_KEY)
            .env("GIT_CONFIG_VALUE_0", "command-line-ide");
        assert_eq!(
            read_ide_with(&mut command_line).unwrap(),
            IdeSelection::Configured("command-line-ide".to_owned())
        );
    }

    #[test]
    fn reports_git_configuration_read_failures() {
        let temporary = TempDir::new();
        let repository = temporary.0.join("repository");
        fs::create_dir_all(&repository).unwrap();
        git(&repository, &["init"]);

        let invalid_config = temporary.0.join("config-directory");
        fs::create_dir_all(&invalid_config).unwrap();
        let mut command = isolated_git(&repository, &temporary);
        command.env("GIT_CONFIG_SYSTEM", invalid_config);

        let error = read_ide_with(&mut command).expect_err("invalid Git config must fail");
        let diagnostic = error.to_string();
        assert!(diagnostic.contains(IDE_CONFIG_KEY));
        assert!(diagnostic.contains("could not read Git configuration key"));
    }

    #[test]
    fn resolves_names_and_atomic_absolute_relative_and_spaced_paths() {
        let temporary = TempDir::new();
        let cursor_name = if cfg!(windows) {
            "cursor.exe"
        } else {
            "cursor"
        };
        let cursor = temporary.0.join(cursor_name);
        make_executable(&cursor);

        let original_path = env::var_os("PATH");
        let joined = env::join_paths(
            std::iter::once(temporary.0.clone()).chain(
                original_path
                    .as_deref()
                    .into_iter()
                    .flat_map(env::split_paths),
            ),
        )
        .unwrap();
        env::set_var("PATH", joined);
        assert_eq!(
            resolve_ide("cursor").unwrap(),
            shell_compatible_absolute(fs::canonicalize(&cursor).unwrap())
        );

        let spaced = temporary.0.join(if cfg!(windows) {
            "Custom IDE.exe"
        } else {
            "Custom IDE"
        });
        make_executable(&spaced);
        assert_eq!(
            resolve_ide(spaced.to_str().unwrap()).unwrap(),
            shell_compatible_absolute(fs::canonicalize(&spaced).unwrap())
        );
        if let Some(path) = original_path {
            env::set_var("PATH", path);
        }
    }

    #[test]
    fn rejects_empty_missing_directories_and_command_templates() {
        let temporary = TempDir::new();
        assert!(resolve_ide("")
            .unwrap_err()
            .to_string()
            .contains(IDE_CONFIG_KEY));
        assert!(resolve_ide(temporary.0.join("missing").to_str().unwrap()).is_err());
        assert!(resolve_ide(temporary.0.to_str().unwrap()).is_err());
        assert!(resolve_ide("cursor --reuse-window").is_err());
        assert!(resolve_ide("cursor && malicious").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_code_resolves_standard_shim_to_gui_with_unicode_and_spaces() {
        let temporary = TempDir::new();
        let install = temporary.0.join("Microsoft VS Cøde");
        let bin = install.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let shim = bin.join("code.cmd");
        fs::write(&shim, "@echo off\r\n").unwrap();
        let gui = install.join("Code.exe");
        make_executable(&gui);

        assert_eq!(
            resolve_ide_selection_with_path(
                &IdeSelection::DefaultCode,
                Platform::Windows,
                Some(bin.as_os_str()),
            )
            .unwrap(),
            shell_compatible_absolute(fs::canonicalize(gui).unwrap())
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_code_accepts_a_direct_gui_executable() {
        let temporary = TempDir::new();
        let gui = temporary.0.join("code.exe");
        make_executable(&gui);

        assert_eq!(
            resolve_ide_selection_with_path(
                &IdeSelection::DefaultCode,
                Platform::Windows,
                Some(temporary.0.as_os_str()),
            )
            .unwrap(),
            shell_compatible_absolute(fs::canonicalize(gui).unwrap())
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_code_rejects_missing_gui_with_shim_and_candidate_context() {
        let temporary = TempDir::new();
        let bin = temporary.0.join("install").join("bin");
        fs::create_dir_all(&bin).unwrap();
        let shim = bin.join("code.cmd");
        fs::write(&shim, "@echo off\r\n").unwrap();
        let candidate = temporary.0.join("install").join("Code.exe");

        let error = resolve_ide_selection_with_path(
            &IdeSelection::DefaultCode,
            Platform::Windows,
            Some(bin.as_os_str()),
        )
        .expect_err("missing GUI executable must fail closed");
        let diagnostic = error.to_string();
        let lowercase_diagnostic = diagnostic.to_lowercase();
        assert!(lowercase_diagnostic.contains(&shim.display().to_string().to_lowercase()));
        assert!(lowercase_diagnostic.contains(&candidate.display().to_string().to_lowercase()));
        assert!(diagnostic.contains("default IDE 'code'"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_code_rejects_batch_fallback() {
        let temporary = TempDir::new();
        let shim = temporary.0.join("code.bat");
        fs::write(&shim, "@echo off\r\n").unwrap();

        let error = resolve_ide_selection_with_path(
            &IdeSelection::DefaultCode,
            Platform::Windows,
            Some(temporary.0.as_os_str()),
        )
        .expect_err("batch fallback must fail closed");
        let diagnostic = error.to_string().to_lowercase();
        assert!(diagnostic.contains("code.bat"));
        assert!(diagnostic.contains("script fallback is disabled"));
        assert!(!diagnostic.contains("cmd.exe /c"));
    }

    #[cfg(windows)]
    #[test]
    fn explicitly_configured_code_keeps_atomic_path_resolution() {
        let temporary = TempDir::new();
        let install = temporary.0.join("install");
        let bin = install.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let shim = bin.join("code.cmd");
        fs::write(&shim, "@echo off\r\n").unwrap();
        make_executable(&install.join("Code.exe"));

        assert_eq!(
            resolve_ide_selection_with_path(
                &IdeSelection::Configured("code".to_owned()),
                Platform::Windows,
                Some(bin.as_os_str()),
            )
            .unwrap(),
            shell_compatible_absolute(fs::canonicalize(shim).unwrap())
        );
    }
}
