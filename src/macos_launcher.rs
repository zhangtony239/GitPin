//! Runtime for the internal launcher embedded in each macOS application bundle.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT_KEY: &str = "X-Git-Pin-Repository-Root";
const IDE_KEY: &str = "X-Git-Pin-Ide-Executable";
const VERSION_KEY: &str = "X-Git-Pin-Format-Version";
const FORMAT_VERSION: &str = "1";
const EXECUTABLE_NAME: &str = "git-pin-launcher";

/// Runs the internal launcher only when the current executable belongs to a
/// fully validated Git Pin application bundle.
///
/// `None` deliberately means "use the public git-pin CLI". A path which only
/// resembles a bundle, or whose managed metadata is incomplete, must never
/// change the public command's argument handling.
pub fn run_if_managed() -> Option<Result<(), String>> {
    let current_executable = std::env::current_exe().ok()?;
    run_if_managed_with(&current_executable, Path::new("/usr/bin/open"))
}

fn run_if_managed_with(
    current_executable: &Path,
    open_executable: &Path,
) -> Option<Result<(), String>> {
    let launch = launch_metadata_for_launcher(current_executable).ok()?;
    Some(launch_repository(&launch, open_executable))
}

/// Reads the owning bundle and opens its repository in the frozen IDE CLI.
pub fn run() -> Result<(), String> {
    let current_executable = std::env::current_exe()
        .map_err(|error| format!("could not locate macOS bundle launcher: {error}"))?;
    run_with(&current_executable, Path::new("/usr/bin/open"))
}

/// Testable launcher entry point with explicit executable paths.
pub fn run_with(current_executable: &Path, open_executable: &Path) -> Result<(), String> {
    let launch = launch_metadata_for_launcher(current_executable)?;
    launch_repository(&launch, open_executable)
}

struct LaunchMetadata {
    root: PathBuf,
    ide_executable: Option<PathBuf>,
}

fn launch_repository(launch: &LaunchMetadata, legacy_open_executable: &Path) -> Result<(), String> {
    let mut command = match &launch.ide_executable {
        Some(ide_executable) => Command::new(ide_executable),
        None => {
            let mut command = Command::new(legacy_open_executable);
            command.arg("-a").arg("Visual Studio Code").arg("--args");
            command
        }
    };
    let status = command.arg(&launch.root).status().map_err(|error| {
        format!(
            "could not launch the configured IDE for repository '{}': {error}",
            launch.root.display()
        )
    })?;
    if !status.success() {
        return Err(format!(
            "macOS IDE launcher failed with status {status} for repository '{}'",
            launch.root.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn repository_root_for_launcher(current_executable: &Path) -> Result<PathBuf, String> {
    Ok(launch_metadata_for_launcher(current_executable)?.root)
}

fn launch_metadata_for_launcher(current_executable: &Path) -> Result<LaunchMetadata, String> {
    if current_executable
        .file_name()
        .and_then(|name| name.to_str())
        != Some(EXECUTABLE_NAME)
    {
        return Err(format!(
            "macOS bundle launcher '{}' does not have the managed executable name '{EXECUTABLE_NAME}'",
            current_executable.display()
        ));
    }
    let executable_metadata = fs::metadata(current_executable).map_err(|error| {
        format!(
            "could not inspect macOS bundle launcher '{}': {error}",
            current_executable.display()
        )
    })?;
    if !executable_metadata.is_file() {
        return Err(format!(
            "macOS bundle launcher '{}' is not a file",
            current_executable.display()
        ));
    }
    let macos_directory = current_executable.parent().ok_or_else(|| {
        format!(
            "macOS bundle launcher '{}' has no parent directory",
            current_executable.display()
        )
    })?;
    if macos_directory.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return Err(format!(
            "macOS bundle launcher '{}' is not inside Contents/MacOS",
            current_executable.display()
        ));
    }
    let contents = macos_directory.parent().ok_or_else(|| {
        format!(
            "macOS bundle launcher '{}' has no Contents directory",
            current_executable.display()
        )
    })?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return Err(format!(
            "macOS bundle launcher '{}' is not inside an app Contents directory",
            current_executable.display()
        ));
    }
    let bundle = contents.parent().ok_or_else(|| {
        format!(
            "macOS bundle launcher '{}' has no owning application bundle",
            current_executable.display()
        )
    })?;
    if bundle.extension().and_then(|extension| extension.to_str()) != Some("app")
        || !bundle.is_dir()
    {
        return Err(format!(
            "macOS bundle launcher '{}' is not inside an .app bundle",
            current_executable.display()
        ));
    }
    let plist_path = contents.join("Info.plist");
    let plist = fs::read_to_string(&plist_path).map_err(|error| {
        format!(
            "could not read managed repository metadata '{}': {error}",
            plist_path.display()
        )
    })?;
    if plist_string(&plist, VERSION_KEY).as_deref() != Some(FORMAT_VERSION) {
        return Err(format!(
            "managed format version is missing or invalid in '{}'",
            plist_path.display()
        ));
    }
    if plist_string(&plist, "CFBundleExecutable").as_deref() != Some(EXECUTABLE_NAME) {
        return Err(format!(
            "managed bundle executable is missing or invalid in '{}'",
            plist_path.display()
        ));
    }
    if plist_string(&plist, "CFBundlePackageType").as_deref() != Some("APPL") {
        return Err(format!(
            "managed bundle package type is missing or invalid in '{}'",
            plist_path.display()
        ));
    }
    let root = plist_string(&plist, ROOT_KEY).ok_or_else(|| {
        format!(
            "managed repository root is missing or invalid in '{}'",
            plist_path.display()
        )
    })?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err(format!(
            "managed repository root '{}' in '{}' is not absolute",
            root.display(),
            plist_path.display()
        ));
    }
    let ide_executable = plist_string(&plist, IDE_KEY).map(PathBuf::from);
    if ide_executable
        .as_ref()
        .is_some_and(|ide_executable| !ide_executable.is_absolute())
    {
        return Err(format!(
            "managed IDE executable in '{}' is not absolute",
            plist_path.display()
        ));
    }
    Ok(LaunchMetadata {
        root,
        ide_executable,
    })
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

#[cfg(test)]
mod tests {
    use super::{repository_root_for_launcher, run_if_managed_with, run_with};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "git-pin-macos-launcher-test-{}-{nonce}",
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

    fn bundle_fixture(temporary: &TempDir) -> PathBuf {
        let contents = temporary.0.join("Project.app/Contents");
        let executable = contents.join("MacOS/git-pin-launcher");
        fs::create_dir_all(executable.parent().expect("launcher must have parent"))
            .expect("bundle directories must be created");
        fs::write(&executable, "launcher fixture").expect("launcher fixture must be written");
        fs::write(
            contents.join("Info.plist"),
            "<plist><dict><key>CFBundleExecutable</key><string>git-pin-launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>X-Git-Pin-Format-Version</key><string>1</string><key>X-Git-Pin-Repository-Root</key><string>/work/项目 &amp; shell;$HOME</string></dict></plist>",
        )
        .expect("plist fixture must be written");
        executable
    }

    fn configurable_bundle_fixture(temporary: &TempDir, ide: &std::path::Path) -> PathBuf {
        let executable = bundle_fixture(temporary);
        let plist = executable
            .parent()
            .expect("launcher must have parent")
            .parent()
            .expect("MacOS must have parent")
            .join("Info.plist");
        let content = fs::read_to_string(&plist).expect("plist fixture must be readable");
        let content = content.replace(
            "</dict>",
            &format!(
                "<key>X-Git-Pin-Ide-Executable</key><string>{}</string></dict>",
                ide.display()
            ),
        );
        fs::write(plist, content).expect("configurable plist fixture must be written");
        executable
    }

    #[test]
    fn reads_repository_root_from_the_owning_bundle() {
        let temporary = TempDir::new();
        let executable = bundle_fixture(&temporary);
        assert_eq!(
            repository_root_for_launcher(&executable).expect("root must be read"),
            PathBuf::from("/work/项目 & shell;$HOME")
        );
    }

    #[test]
    fn invokes_open_with_an_argument_array_without_shell_interpolation() {
        let temporary = TempDir::new();
        let executable = bundle_fixture(&temporary);
        let output = temporary.0.join("arguments.txt");
        let fake_open = temporary.0.join("open");
        fs::write(
            &fake_open,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
                output.display()
            ),
        )
        .expect("fake open must be written");
        fs::set_permissions(&fake_open, fs::Permissions::from_mode(0o755))
            .expect("fake open must be executable");

        run_with(&executable, &fake_open).expect("launcher must succeed");
        assert_eq!(
            fs::read_to_string(output).expect("captured arguments must be readable"),
            "-a\nVisual Studio Code\n--args\n/work/项目 & shell;$HOME\n"
        );
    }

    #[test]
    fn invokes_the_frozen_ide_directly_with_one_repository_argument() {
        let temporary = TempDir::new();
        let output = temporary.0.join("captured arguments.txt");
        let ide = temporary.0.join("Cursor 编辑器").join("cursor");
        fs::create_dir_all(ide.parent().expect("IDE fixture must have a parent"))
            .expect("IDE fixture directory must be created");
        fs::write(
            &ide,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
                output.display()
            ),
        )
        .expect("IDE fixture must be written");
        fs::set_permissions(&ide, fs::Permissions::from_mode(0o755))
            .expect("IDE fixture must be executable");
        let executable = configurable_bundle_fixture(&temporary, &ide);

        run_with(&executable, PathBuf::from("/missing/legacy-open").as_path())
            .expect("launcher must directly execute the configured IDE");
        assert_eq!(
            fs::read_to_string(output).expect("captured arguments must be readable"),
            "/work/项目 & shell;$HOME\n"
        );
    }

    #[test]
    fn dispatches_only_fully_validated_managed_bundles() {
        let temporary = TempDir::new();
        let executable = bundle_fixture(&temporary);
        let fake_open = temporary.0.join("open");
        fs::write(&fake_open, "#!/bin/sh\nexit 0\n").expect("fake open must be written");
        fs::set_permissions(&fake_open, fs::Permissions::from_mode(0o755))
            .expect("fake open must be executable");

        assert_eq!(run_if_managed_with(&executable, &fake_open), Some(Ok(())));

        let plist = executable
            .parent()
            .expect("launcher must have parent")
            .parent()
            .expect("MacOS must have parent")
            .join("Info.plist");
        fs::write(
            plist,
            "<plist><dict><key>X-Git-Pin-Repository-Root</key><string>/work/project</string></dict></plist>",
        )
        .expect("invalid plist fixture must be written");
        assert_eq!(run_if_managed_with(&executable, &fake_open), None);
    }

    #[test]
    fn rejects_lookalike_paths_and_executable_names() {
        let temporary = TempDir::new();
        let executable = bundle_fixture(&temporary);
        let wrong_name = executable.with_file_name("git-pin");
        fs::write(&wrong_name, "launcher fixture").expect("lookalike fixture must be written");
        assert!(repository_root_for_launcher(&wrong_name).is_err());

        let wrong_structure = temporary.0.join("Project/Contents/MacOS/git-pin-launcher");
        fs::create_dir_all(
            wrong_structure
                .parent()
                .expect("lookalike launcher must have parent"),
        )
        .expect("lookalike directories must be created");
        fs::write(&wrong_structure, "launcher fixture")
            .expect("lookalike launcher must be written");
        assert!(repository_root_for_launcher(&wrong_structure).is_err());
    }
}
