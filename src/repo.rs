//! Git repository discovery and naming.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

/// Platform rules that affect launcher names and path identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Windows,
    Linux,
    MacOs,
}

impl Platform {
    /// Returns the platform selected by the current compilation target.
    pub const fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        compile_error!("git-pin supports only Windows, Linux, and macOS");
    }
}

/// Asks Git for the absolute top-level working tree containing `input`.
pub fn discover_root(input: &Path) -> Result<PathBuf, AppError> {
    discover_root_with_git(input, OsStr::new("git"))
}

/// Derives and validates the exact user-visible repository basename.
pub fn repository_name(root: &Path, platform: Platform) -> Result<String, AppError> {
    let basename = root.file_name().ok_or_else(|| {
        AppError::failure(format!(
            "repository root '{}' has no usable basename",
            root.display()
        ))
    })?;
    let name = basename.to_str().ok_or_else(|| {
        AppError::failure(format!(
            "repository basename for '{}' is not valid UTF-8",
            root.display()
        ))
    })?;

    validate_name(name, platform).map_err(|reason| {
        AppError::failure(format!(
            "repository name '{name}' cannot be represented safely on {platform:?}: {reason}"
        ))
    })?;
    Ok(name.to_owned())
}

fn validate_name(name: &str, platform: Platform) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("the name is empty");
    }
    if name == "." || name == ".." {
        return Err("dot path components are not launcher names");
    }
    if name
        .chars()
        .any(|character| character == '\0' || character == '/')
    {
        return Err("the name contains a path separator or NUL");
    }

    match platform {
        Platform::Linux => Ok(()),
        Platform::MacOs if name.contains(':') => Err("the name contains ':'"),
        Platform::MacOs => Ok(()),
        Platform::Windows => validate_windows_name(name),
    }
}

fn validate_windows_name(name: &str) -> Result<(), &'static str> {
    if name.ends_with([' ', '.']) {
        return Err("Windows names cannot end in a space or period");
    }
    if name
        .chars()
        .any(|character| character.is_control() || r#"<>:"\|?*"#.contains(character))
    {
        return Err("the name contains a character forbidden by Windows");
    }

    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        return Err("the name is reserved by Windows");
    }

    Ok(())
}

/// Compares normalized repository roots using the selected platform semantics.
pub fn paths_equivalent(left: &Path, right: &Path, platform: Platform) -> bool {
    let canonical = match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => Some((left, right)),
        _ => None,
    };

    match (platform, canonical) {
        (Platform::Windows, Some((left, right))) => {
            windows_path_key(&left) == windows_path_key(&right)
        }
        (Platform::Windows, None) => windows_path_key(left) == windows_path_key(right),
        (Platform::Linux | Platform::MacOs, Some((left, right))) => left == right,
        (Platform::Linux | Platform::MacOs, None) => left == right,
    }
}

fn windows_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn discover_root_with_git(input: &Path, git: &OsStr) -> Result<PathBuf, AppError> {
    let output = Command::new(git)
        .arg("-C")
        .arg(input)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| {
            AppError::failure(format!(
                "could not run Git while discovering repository from '{}': {error}",
                input.display()
            ))
        })?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::failure(format!(
            "could not discover a Git working tree from '{}': {}",
            input.display(),
            detail.trim()
        )));
    }

    let root_text = String::from_utf8(output.stdout).map_err(|_| {
        AppError::failure(format!(
            "Git returned a repository path that is not valid UTF-8 for '{}'",
            input.display()
        ))
    })?;
    let root = PathBuf::from(root_text.trim_end_matches(['\r', '\n']));

    if root.as_os_str().is_empty() || !root.is_absolute() {
        return Err(AppError::failure(format!(
            "Git did not return an absolute repository root for '{}': '{}'",
            input.display(),
            root.display()
        )));
    }

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::{
        discover_root, discover_root_with_git, paths_equivalent, repository_name, validate_name,
        Platform,
    };
    use crate::error::ExitCode;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("git-pin-{label}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("temporary directory must be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(arguments)
            .output()
            .expect("Git must be available in CI");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repository(label: &str) -> TempDir {
        let directory = TempDir::new(label);
        git(directory.path(), &["init"]);
        directory
    }

    #[test]
    fn discovers_root_from_the_current_repository_directory() {
        let repository = repository("current");
        let root = discover_root(repository.path()).expect("repository must be discovered");
        assert!(paths_equivalent(
            &root,
            repository.path(),
            Platform::current()
        ));
    }

    #[test]
    fn discovers_root_from_a_given_subdirectory() {
        let repository = repository("subdirectory");
        let nested = repository.path().join("one").join("two");
        fs::create_dir_all(&nested).expect("nested directory must be created");

        let root = discover_root(&nested).expect("repository must be discovered");
        assert!(paths_equivalent(
            &root,
            repository.path(),
            Platform::current()
        ));
    }

    #[test]
    fn discovers_a_linked_worktree_root() {
        let repository = repository("worktree-source");
        git(repository.path(), &["config", "user.name", "Git Pin CI"]);
        git(
            repository.path(),
            &["config", "user.email", "git-pin@example.invalid"],
        );
        fs::write(repository.path().join("tracked"), "content").expect("fixture must be written");
        git(repository.path(), &["add", "tracked"]);
        git(repository.path(), &["commit", "-m", "fixture"]);

        let worktree_parent = TempDir::new("worktree-parent");
        let worktree = worktree_parent.path().join("linked");
        let worktree_text = worktree.to_string_lossy().into_owned();
        git(
            repository.path(),
            &["worktree", "add", "--detach", &worktree_text],
        );

        let root = discover_root(&worktree).expect("worktree must be discovered");
        assert!(paths_equivalent(&root, &worktree, Platform::current()));
    }

    #[test]
    fn reports_non_repository_input_with_context() {
        let directory = TempDir::new("not-a-repository");
        let error = discover_root(directory.path()).expect_err("discovery must fail");
        assert_eq!(error.exit_code(), ExitCode::Failure);
        assert!(error
            .to_string()
            .contains(&directory.path().display().to_string()));
        assert!(error.to_string().contains("Git working tree"));
    }

    #[test]
    fn reports_when_git_is_unavailable() {
        let directory = TempDir::new("git-unavailable");
        let error = discover_root_with_git(
            directory.path(),
            OsStr::new("git-pin-command-that-does-not-exist"),
        )
        .expect_err("missing Git must fail");
        assert_eq!(error.exit_code(), ExitCode::Failure);
        assert!(error.to_string().contains("could not run Git"));
        assert!(error
            .to_string()
            .contains(&directory.path().display().to_string()));
    }

    #[test]
    fn preserves_spaces_and_non_ascii_repository_names() {
        for platform in [Platform::Windows, Platform::Linux, Platform::MacOs] {
            assert_eq!(
                repository_name(Path::new("/projects/项目 with spaces"), platform)
                    .expect("name must be safe"),
                "项目 with spaces"
            );
        }
    }

    #[test]
    fn rejects_platform_specific_unsafe_names_without_renaming() {
        for name in ["bad/name", ".", ".."] {
            for platform in [Platform::Windows, Platform::Linux, Platform::MacOs] {
                assert!(validate_name(name, platform).is_err());
            }
        }

        for name in ["trailing.", "trailing ", "CON", "a<b", "a\\b"] {
            assert!(validate_name(name, Platform::Windows).is_err());
        }
        assert!(validate_name("a:b", Platform::MacOs).is_err());
        assert!(validate_name("a:b", Platform::Linux).is_ok());
    }

    #[test]
    fn compares_paths_with_platform_specific_case_rules() {
        let upper = Path::new("C:/Users/Example/Repo");
        let lower = Path::new("c:\\users\\example\\repo\\");
        assert!(paths_equivalent(upper, lower, Platform::Windows));
        assert!(!paths_equivalent(upper, lower, Platform::Linux));
        assert!(!paths_equivalent(upper, lower, Platform::MacOs));
    }
}
