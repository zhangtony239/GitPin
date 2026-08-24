use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "git-pin-process-{label}-{}-{nonce}",
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

fn git_pin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_git-pin"))
}

fn git_unpin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_git-unpin"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("process must start")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "process failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_with(output: &Output, expected: &str) {
    assert!(!output.status.success(), "process unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "stderr did not contain {expected:?}: {stderr}"
    );
}

fn git(directory: &Path, arguments: &[&str]) {
    let output = run(Command::new("git").arg("-C").arg(directory).args(arguments));
    assert_success(&output);
}

fn initialize_repository(path: &Path) {
    fs::create_dir_all(path).expect("repository directory must be created");
    git(path, &["init"]);
}

struct ProcessEnvironment {
    root: TempDir,
    path: OsString,
}

impl ProcessEnvironment {
    fn new(label: &str) -> Self {
        let root = TempDir::new(label);
        let binary_directory = git_pin()
            .parent()
            .expect("binary must have a parent")
            .to_owned();
        let tools = root.0.join("tools");
        fs::create_dir_all(&tools).expect("tools directory must be created");
        create_vscode_fixture(&tools, &root.0);
        let path = env::join_paths(
            [binary_directory, tools]
                .into_iter()
                .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
        )
        .expect("test PATH must be valid");
        Self { root, path }
    }

    fn command(&self, executable: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(executable);
        command.env("PATH", &self.path);

        #[cfg(target_os = "linux")]
        {
            command.env("XDG_DATA_HOME", self.root.0.join("data"));
            command.env("HOME", self.root.0.join("home"));
        }
        #[cfg(target_os = "windows")]
        {
            command.env("APPDATA", self.root.0.join("appdata"));
            command.env("LOCALAPPDATA", self.root.0.join("localappdata"));
        }
        #[cfg(target_os = "macos")]
        {
            command.env("HOME", self.root.0.join("home"));
        }
        command
    }

    fn launcher(&self, name: &str) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            self.root
                .0
                .join("data/applications")
                .join(format!("git-pin-{name}.desktop"))
        }
        #[cfg(target_os = "windows")]
        {
            self.root
                .0
                .join("appdata/Microsoft/Windows/Start Menu/Programs/.pinned_repo")
                .join(format!("{name}.lnk"))
        }
        #[cfg(target_os = "macos")]
        {
            self.root
                .0
                .join("home/Applications/Git Pin")
                .join(format!("{name}.app"))
        }
    }

    fn launcher_root(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            self.root.0.join("data/applications")
        }
        #[cfg(target_os = "windows")]
        {
            self.root
                .0
                .join("appdata/Microsoft/Windows/Start Menu/Programs/.pinned_repo")
        }
        #[cfg(target_os = "macos")]
        {
            self.root.0.join("home/Applications/Git Pin")
        }
    }

    fn assert_no_launcher_residue(&self) {
        let root = self.launcher_root();
        if !root.exists() {
            return;
        }
        let residue: Vec<_> = root
            .read_dir()
            .expect("launcher root must be readable")
            .map(|entry| entry.expect("launcher entry must be readable").path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .expect("launcher entry must have a name")
                    .to_string_lossy();
                name.contains(".tmp")
                    || name.ends_with(".lnk")
                    || name.ends_with(".app")
                    || name.ends_with(".desktop")
            })
            .collect();
        assert!(residue.is_empty(), "launcher residue remained: {residue:?}");
    }
}

#[cfg(target_os = "linux")]
fn create_vscode_fixture(tools: &Path, _root: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let code = tools.join("code");
    fs::write(&code, "#!/bin/sh\nexit 0\n").expect("Code fixture must be written");
    fs::set_permissions(code, fs::Permissions::from_mode(0o755))
        .expect("Code fixture must be executable");
}

#[cfg(target_os = "windows")]
fn create_vscode_fixture(tools: &Path, _root: &Path) {
    fs::copy(git_pin(), tools.join("code.exe"))
        .expect("a valid PE executable must be copied as the Code fixture");
}

#[cfg(target_os = "macos")]
fn create_vscode_fixture(_tools: &Path, root: &Path) {
    fs::create_dir_all(root.join("home/Applications/Visual Studio Code.app"))
        .expect("isolated user-level Visual Studio Code fixture must be created");
}

#[test]
fn both_binaries_reject_options_and_extra_arguments_with_usage_exit_code() {
    for (binary, usage) in [
        (git_pin(), "usage: git pin [path]"),
        (git_unpin(), "usage: git unpin [path|name]"),
    ] {
        for arguments in [vec!["--all"], vec!["one", "two"]] {
            let output = run(Command::new(&binary).args(arguments));
            assert_eq!(output.status.code(), Some(2));
            assert_failure_with(&output, usage);
        }
    }
}

#[test]
fn git_external_dispatch_and_direct_binaries_cover_the_repository_lifecycle() {
    let environment = ProcessEnvironment::new("lifecycle");
    let first = environment.root.0.join("first/shared-name");
    let second = environment.root.0.join("second/shared-name");
    initialize_repository(&first);
    initialize_repository(&second);
    let nested = first.join("nested directory");
    fs::create_dir_all(&nested).expect("nested repository directory must be created");

    let output = run(environment
        .command("git")
        .current_dir(&nested)
        .args(["pin"]));
    assert_success(&output);
    let launcher = environment.launcher("shared-name");
    assert!(launcher.exists(), "native launcher must be created");

    let output = run(environment.command(git_pin()).arg(&first));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("already pinned"));

    let output = run(environment.command(git_pin()).arg(&second));
    assert_failure_with(&output, "already points to");
    assert!(
        launcher.exists(),
        "conflict must preserve the original launcher"
    );

    let output = run(environment.command(git_unpin()).arg(&first));
    assert_success(&output);
    assert!(!launcher.exists(), "path unpin must remove the launcher");

    assert_success(&run(environment.command(git_pin()).arg(&first)));
    let output = run(environment.command(git_unpin()).arg("shared-name"));
    assert_success(&output);
    assert!(!launcher.exists(), "name unpin must remove the launcher");

    let output = run(environment.command(git_unpin()).arg("shared-name"));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("already unpinned"));
    environment.assert_no_launcher_residue();
}

#[test]
fn repository_errors_include_context_and_do_not_create_launchers() {
    let environment = ProcessEnvironment::new("not-repository");
    let directory = environment.root.0.join("plain directory");
    fs::create_dir_all(&directory).expect("plain directory must be created");

    let output = run(environment.command(git_pin()).arg(&directory));
    assert_failure_with(&output, "could not discover a Git working tree");
    assert!(
        !environment.launcher("plain directory").exists(),
        "repository failure must not create a launcher"
    );
}
