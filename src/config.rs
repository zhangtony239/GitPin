//! Git-backed configuration and IDE executable resolution.

use std::env;
use std::ffi::OsStr;
#[cfg(windows)]
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

pub const IDE_CONFIG_KEY: &str = "pin.ide";
pub const DEFAULT_IDE: &str = "code";

/// Reads the effective IDE value using Git's own configuration machinery.
pub fn read_ide() -> Result<String, AppError> {
    let mut command = Command::new("git");
    read_ide_with(&mut command)
}

fn read_ide_with(command: &mut Command) -> Result<String, AppError> {
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
        return Ok(value.trim_end_matches(['\r', '\n']).to_owned());
    }

    if output.status.code() == Some(1) && output.stderr.is_empty() {
        return Ok(DEFAULT_IDE.to_owned());
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
        find_on_path(configured.as_os_str()).ok_or_else(|| {
            invalid_ide(
                value,
                "no executable with that exact name was found in PATH; values are not split into a command and arguments",
            )
        })?
    };

    validate_and_absolute(value, &candidate)
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

fn find_on_path(name: &OsStr) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
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
        read_ide_with, resolve_ide, shell_compatible_absolute, DEFAULT_IDE, IDE_CONFIG_KEY,
    };
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
            DEFAULT_IDE
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
            "repository-ide"
        );

        let mut command_line = isolated_git(&repository, &temporary);
        command_line
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", IDE_CONFIG_KEY)
            .env("GIT_CONFIG_VALUE_0", "command-line-ide");
        assert_eq!(
            read_ide_with(&mut command_line).unwrap(),
            "command-line-ide"
        );
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
}
