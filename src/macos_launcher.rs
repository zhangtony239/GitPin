//! Runtime for the internal launcher embedded in each macOS application bundle.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT_KEY: &str = "X-Git-Pin-Repository-Root";

/// Reads the owning bundle and opens its repository in stable Visual Studio Code.
pub fn run() -> Result<(), String> {
    let current_executable = std::env::current_exe()
        .map_err(|error| format!("could not locate macOS bundle launcher: {error}"))?;
    run_with(&current_executable, Path::new("/usr/bin/open"))
}

/// Testable launcher entry point with explicit executable paths.
pub fn run_with(current_executable: &Path, open_executable: &Path) -> Result<(), String> {
    let root = repository_root_for_launcher(current_executable)?;
    let status = Command::new(open_executable)
        .arg("-a")
        .arg("Visual Studio Code")
        .arg("--args")
        .arg(&root)
        .status()
        .map_err(|error| {
            format!(
                "could not launch Visual Studio Code for repository '{}': {error}",
                root.display()
            )
        })?;
    if !status.success() {
        return Err(format!(
            "macOS open failed with status {status} for repository '{}'",
            root.display()
        ));
    }
    Ok(())
}

pub(crate) fn repository_root_for_launcher(current_executable: &Path) -> Result<PathBuf, String> {
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
    let plist_path = contents.join("Info.plist");
    let plist = fs::read_to_string(&plist_path).map_err(|error| {
        format!(
            "could not read managed repository metadata '{}': {error}",
            plist_path.display()
        )
    })?;
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
    Ok(root)
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
    use super::{repository_root_for_launcher, run_with};
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
        fs::write(
            contents.join("Info.plist"),
            "<plist><dict><key>X-Git-Pin-Repository-Root</key><string>/work/项目 &amp; shell;$HOME</string></dict></plist>",
        )
        .expect("plist fixture must be written");
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
}
