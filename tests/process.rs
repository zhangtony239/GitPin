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
        command.env("GIT_CONFIG_SYSTEM", self.root.0.join("system.gitconfig"));
        command.env("GIT_CONFIG_GLOBAL", self.root.0.join("global.gitconfig"));

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

    fn create_ide(&self, name: &str) -> PathBuf {
        let tools = self.root.0.join("tools");
        let file_name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        let executable = tools.join(file_name);
        create_ide_fixture(&executable);
        git_pin::config::resolve_ide(executable.to_str().expect("fixture path must be UTF-8"))
            .expect("IDE fixture must resolve like production configuration")
    }
}

#[cfg(unix)]
fn create_vscode_fixture(tools: &Path, _root: &Path) {
    let code = tools.join("code");
    create_ide_fixture(&code);
}

#[cfg(target_os = "windows")]
fn create_vscode_fixture(tools: &Path, _root: &Path) {
    fs::copy(git_pin(), tools.join("code.exe"))
        .expect("a valid PE executable must be copied as the Code fixture");
}

#[cfg(unix)]
fn create_ide_fixture(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, "#!/bin/sh\nexit 0\n").expect("IDE fixture must be written");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("IDE fixture must be executable");
}

#[cfg(windows)]
fn create_ide_fixture(path: &Path) {
    fs::copy(git_pin(), path).expect("a valid PE executable must be copied as the IDE fixture");
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
fn help_is_successful_complete_and_has_no_launcher_side_effects() {
    let environment = ProcessEnvironment::new("help");
    let mut expected_stdout = None;
    for argument in ["--help", "-h"] {
        let output = run(environment.command(git_pin()).arg(argument));
        assert_success(&output);
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "git pin [path]",
            "git pin --list",
            "git pin --prune",
            "pin.ide",
            "default is `code`",
            "ide path/to/repo",
            "git pin -h",
            "git-pin --help",
        ] {
            assert!(
                stdout.contains(expected),
                "help omitted {expected:?}: {stdout}"
            );
        }
        if let Some(expected) = &expected_stdout {
            assert_eq!(&output.stdout, expected);
        } else {
            expected_stdout = Some(output.stdout);
        }
    }

    let git_dispatched = run(environment.command("git").args(["pin", "-h"]));
    assert_success(&git_dispatched);
    assert!(git_dispatched.stderr.is_empty());
    assert_eq!(git_dispatched.stdout, expected_stdout.unwrap());
    environment.assert_no_launcher_residue();
}

#[test]
fn git_dispatch_preserves_effective_ide_configuration_precedence() {
    let environment = ProcessEnvironment::new("git-config");
    let repository = environment.root.0.join("repositories/configured-project");
    initialize_repository(&repository);
    let global_ide = environment.create_ide("global-cursor");
    let repository_ide = environment.create_ide("repository-cursor");
    let command_line_ide = environment.create_ide("command-line-cursor");

    assert_success(&run(environment
        .command("git")
        .args(["config", "--global", "pin.ide"])
        .arg(&global_ide)));
    assert_success(&run(environment
        .command("git")
        .current_dir(&repository)
        .args(["config", "pin.ide"])
        .arg(&repository_ide)));

    let output = run(environment.command("git").current_dir(&repository).args([
        "-c",
        &format!("pin.ide={}", command_line_ide.display()),
        "pin",
    ]));
    assert_success(&output);
    let launcher = environment.launcher("configured-project");
    assert!(launcher.exists());
    assert_launcher_contains_path(&launcher, &command_line_ide);

    assert_success(&run(environment.command(git_unpin()).arg(&repository)));
    assert_success(&run(environment
        .command("git")
        .current_dir(&repository)
        .arg("pin")));
    assert_launcher_contains_path(&launcher, &repository_ide);

    assert_success(&run(environment.command(git_unpin()).arg(&repository)));
    assert_success(&run(environment
        .command("git")
        .current_dir(&repository)
        .args(["config", "--unset", "pin.ide"])));
    assert_success(&run(environment
        .command("git")
        .current_dir(&repository)
        .arg("pin")));
    assert_launcher_contains_path(&launcher, &global_ide);
}

fn assert_launcher_contains_path(launcher: &Path, expected: &Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let bytes = fs::read(launcher).expect("Shell Link must be readable");
        let expected_display = expected.display().to_string();
        let expected_bytes: Vec<u8> = expected
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(
            bytes
                .windows(expected_bytes.len())
                .any(|window| window == expected_bytes),
            "Shell Link did not freeze expected IDE path {}",
            expected_display
        );
    }
    #[cfg(target_os = "linux")]
    {
        let content = fs::read_to_string(launcher).expect("Desktop Entry must be readable");
        assert!(content.contains(&expected.display().to_string()));
    }
    #[cfg(target_os = "macos")]
    {
        let content = fs::read_to_string(launcher.join("Contents/Info.plist"))
            .expect("bundle metadata must be readable");
        assert!(content.contains(&expected.display().to_string()));
    }
}

#[test]
fn list_is_read_only_and_prune_is_diagnostic_and_idempotent() {
    let environment = ProcessEnvironment::new("maintenance");
    let repository = environment.root.0.join("repositories/stale-project");
    initialize_repository(&repository);
    assert_success(&run(environment.command(git_pin()).arg(&repository)));
    let launcher = environment.launcher("stale-project");
    assert!(launcher.exists());

    let output = run(environment.command(git_pin()).arg("--list"));
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stale-project"));
    assert!(stdout.contains("valid"));
    assert!(launcher.exists(), "list must not modify a launcher");

    fs::remove_dir_all(&repository).expect("repository fixture must be deleted");
    let output = run(environment.command(git_pin()).arg("--list"));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("invalid"));
    assert!(
        launcher.exists(),
        "invalid list item must remain until prune"
    );

    let output = run(environment.command(git_pin()).arg("--prune"));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("pruned 'stale-project'"));
    assert!(!launcher.exists());

    let output = run(environment.command(git_pin()).arg("--prune"));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("no stale pinned repositories"));
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

#[cfg(target_os = "macos")]
#[test]
fn macos_two_binary_release_is_self_contained_and_preserves_public_commands() {
    use std::os::unix::fs::PermissionsExt;

    let environment = ProcessEnvironment::new("macos-self-contained");
    let release = environment.root.0.join("release");
    fs::create_dir_all(&release).expect("simulated release directory must be created");
    let release_pin = release.join("git-pin");
    let release_unpin = release.join("git-unpin");
    fs::copy(git_pin(), &release_pin).expect("git-pin must be staged");
    fs::copy(git_unpin(), &release_unpin).expect("git-unpin must be staged");
    for binary in [&release_pin, &release_unpin] {
        fs::set_permissions(binary, fs::Permissions::from_mode(0o755))
            .expect("staged binary must be executable");
    }
    let mut staged_files = release
        .read_dir()
        .expect("release directory must be readable")
        .map(|entry| entry.expect("release entry must be readable").file_name())
        .collect::<Vec<_>>();
    staged_files.sort();
    assert_eq!(
        staged_files,
        [OsString::from("git-pin"), OsString::from("git-unpin")]
    );

    for (binary, usage) in [
        (&release_pin, "usage: git pin [path]"),
        (&release_unpin, "usage: git unpin [path|name]"),
    ] {
        let output = run(environment.command(binary).arg("--all"));
        assert_eq!(output.status.code(), Some(2));
        assert_failure_with(&output, usage);
    }

    let repository = environment
        .root
        .0
        .join("repositories/project with spaces & shell;$HOME");
    initialize_repository(&repository);
    let canonical_repository = fs::canonicalize(&repository)
        .expect("repository path must have an absolute canonical form");
    let release_path = env::join_paths(
        std::iter::once(release.clone()).chain(env::split_paths(&environment.path)),
    )
    .expect("release PATH must be valid");

    let output = run(environment
        .command("git")
        .env("PATH", &release_path)
        .current_dir(&repository)
        .arg("pin"));
    assert_success(&output);
    let bundle = environment.launcher("project with spaces & shell;$HOME");
    let embedded = bundle.join("Contents/MacOS/git-pin-launcher");
    assert!(
        embedded.is_file(),
        "bundle must contain an executable entry"
    );
    assert_eq!(
        fs::read(&embedded).expect("embedded executable must be readable"),
        fs::read(&release_pin).expect("staged git-pin must be readable")
    );

    let fake_open = environment.root.0.join("fake-open/open");
    fs::create_dir_all(fake_open.parent().expect("fake open must have a parent"))
        .expect("fake open directory must be created");
    fs::write(
        &fake_open,
        "#!/bin/sh\nscript_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nprintf '%s\\n' \"$@\" > \"$script_dir/arguments.txt\"\n",
    )
    .expect("fake open must be written");
    fs::set_permissions(&fake_open, fs::Permissions::from_mode(0o755))
        .expect("fake open must be executable");
    git_pin::macos_launcher::run_with(&embedded, &fake_open)
        .expect("embedded entry must read and launch the managed root");
    assert_eq!(
        fs::read_to_string(
            fake_open
                .parent()
                .expect("fake open must have a parent")
                .join("arguments.txt")
        )
        .expect("captured open arguments must be readable"),
        format!(
            "-a\nVisual Studio Code\n--args\n{}\n",
            canonical_repository.display()
        )
    );

    let output = run(environment
        .command("git")
        .env("PATH", &release_path)
        .current_dir(&repository)
        .arg("pin"));
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("already pinned"));

    let output = run(environment
        .command("git")
        .env("PATH", &release_path)
        .current_dir(&repository)
        .arg("unpin"));
    assert_success(&output);
    assert!(!bundle.exists(), "public git unpin must remove the bundle");
    environment.assert_no_launcher_residue();
}
