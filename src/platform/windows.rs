use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, STGM,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

use crate::error::AppError;
use crate::launcher::{
    CreateOutcome, LauncherBackend, LauncherEnumerationError, LauncherEnumerationItem,
    LauncherInspection, LauncherRoot, ManagedLauncher,
};
use crate::repo::{paths_equivalent, Platform, Repository};

const FORMAT_DESCRIPTION: &str = "Git Pin managed launcher v1";
const WIDE_BUFFER_LENGTH: usize = 32_768;

/// Owns one successful COM initialization on the current thread.
struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, AppError> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|error| {
                AppError::failure(format!(
                    "could not initialize the Windows Shell COM apartment: {error}"
                ))
            })?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

/// Windows implementation backed by Start Menu Shell Links.
pub struct WindowsBackend {
    root: LauncherRoot,
}

impl WindowsBackend {
    pub fn new() -> Result<Self, AppError> {
        let app_data = env::var_os("APPDATA").ok_or_else(|| {
            AppError::failure(
                "could not determine Windows launcher directory because APPDATA is not set",
            )
        })?;
        let root = PathBuf::from(app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(".pinned_repo");
        Ok(Self {
            root: LauncherRoot::system(root),
        })
    }

    #[cfg(test)]
    fn for_test(root: PathBuf) -> Self {
        Self {
            root: LauncherRoot::for_test(root),
        }
    }

    fn launcher_path(&self, name: &str) -> PathBuf {
        self.root.as_path().join(format!("{name}.lnk"))
    }

    fn inspect_path(&self, name: &str, path: PathBuf) -> Result<LauncherInspection, AppError> {
        if !path.exists() {
            return Ok(LauncherInspection::Missing);
        }
        if !path.is_file() {
            return Ok(LauncherInspection::Foreign { path });
        }

        let shortcut = match read_shell_link(&path) {
            Ok(shortcut) => shortcut,
            Err(_) => return Ok(LauncherInspection::Foreign { path }),
        };
        if shortcut.description != OsStr::new(FORMAT_DESCRIPTION)
            || !shortcut.target.is_absolute()
            || !shortcut.icon.is_absolute()
            || (shortcut.target.exists()
                && !paths_equivalent(&shortcut.icon, &shortcut.target, Platform::Windows))
            || !shortcut.root.is_absolute()
            || shortcut.arguments != quote_single_argument(shortcut.root.as_os_str())
        {
            return Ok(LauncherInspection::Foreign { path });
        }

        Ok(LauncherInspection::Managed(ManagedLauncher {
            name: name.to_owned(),
            root: shortcut.root,
            ide_executable: shortcut.target,
            path,
        }))
    }
}

struct ShellLinkData {
    target: PathBuf,
    arguments: OsString,
    root: PathBuf,
    icon: PathBuf,
    description: OsString,
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide_buffer(buffer: &[u16]) -> OsString {
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..length])
}

fn quote_single_argument(argument: &OsStr) -> OsString {
    let mut quoted = Vec::new();
    quoted.push('"' as u16);
    let mut backslashes = 0_usize;

    for character in argument.encode_wide() {
        if character == '\\' as u16 {
            backslashes += 1;
        } else if character == '"' as u16 {
            quoted.extend(std::iter::repeat_n('\\' as u16, backslashes * 2 + 1));
            quoted.push(character);
            backslashes = 0;
        } else {
            quoted.extend(std::iter::repeat_n('\\' as u16, backslashes));
            quoted.push(character);
            backslashes = 0;
        }
    }
    quoted.extend(std::iter::repeat_n('\\' as u16, backslashes * 2));
    quoted.push('"' as u16);
    OsString::from_wide(&quoted)
}

fn create_shell_link(
    path: &Path,
    repository: &Repository,
    ide_executable: &Path,
) -> Result<(), AppError> {
    let link: IShellLinkW = unsafe {
        CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|error| {
            AppError::failure(format!(
                "could not create Windows Shell Link COM object: {error}"
            ))
        })?
    };

    let target = wide(ide_executable.as_os_str());
    let arguments = wide(&quote_single_argument(repository.root().as_os_str()));
    let working_directory = wide(repository.root().as_os_str());
    let description = wide(OsStr::new(FORMAT_DESCRIPTION));
    unsafe {
        link.SetPath(PCWSTR(target.as_ptr())).map_err(|error| {
            AppError::failure(format!("could not set Shell Link target: {error}"))
        })?;
        link.SetArguments(PCWSTR(arguments.as_ptr()))
            .map_err(|error| {
                AppError::failure(format!(
                    "could not set Shell Link repository argument: {error}"
                ))
            })?;
        link.SetWorkingDirectory(PCWSTR(working_directory.as_ptr()))
            .map_err(|error| {
                AppError::failure(format!(
                    "could not set Shell Link working directory: {error}"
                ))
            })?;
        link.SetIconLocation(PCWSTR(target.as_ptr()), 0)
            .map_err(|error| {
                AppError::failure(format!("could not set Shell Link icon: {error}"))
            })?;
        link.SetDescription(PCWSTR(description.as_ptr()))
            .map_err(|error| {
                AppError::failure(format!(
                    "could not set Shell Link managed metadata: {error}"
                ))
            })?;
    }

    let persist: IPersistFile = link.cast().map_err(|error| {
        AppError::failure(format!(
            "could not query IPersistFile for Windows Shell Link: {error}"
        ))
    })?;
    let output = wide(path.as_os_str());
    unsafe {
        persist
            .Save(PCWSTR(output.as_ptr()), true)
            .map_err(|error| {
                AppError::failure(format!(
                    "could not save Windows Shell Link '{}': {error}",
                    path.display()
                ))
            })?;
        persist
            .SaveCompleted(PCWSTR(output.as_ptr()))
            .map_err(|error| {
                AppError::failure(format!(
                    "could not complete Windows Shell Link save '{}': {error}",
                    path.display()
                ))
            })?;
    }
    Ok(())
}

fn read_shell_link(path: &Path) -> Result<ShellLinkData, AppError> {
    let link: IShellLinkW = unsafe {
        CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|error| {
            AppError::failure(format!(
                "could not create Windows Shell Link COM object: {error}"
            ))
        })?
    };
    let persist: IPersistFile = link.cast().map_err(|error| {
        AppError::failure(format!(
            "could not query IPersistFile for Windows Shell Link: {error}"
        ))
    })?;
    let input = wide(path.as_os_str());
    unsafe {
        persist
            .Load(PCWSTR(input.as_ptr()), STGM(0))
            .map_err(|error| {
                AppError::failure(format!(
                    "could not load Windows Shell Link '{}': {error}",
                    path.display()
                ))
            })?;
    }

    let mut target = vec![0_u16; WIDE_BUFFER_LENGTH];
    let mut arguments = vec![0_u16; WIDE_BUFFER_LENGTH];
    let mut working_directory = vec![0_u16; WIDE_BUFFER_LENGTH];
    let mut icon = vec![0_u16; WIDE_BUFFER_LENGTH];
    let mut description = vec![0_u16; WIDE_BUFFER_LENGTH];
    let mut icon_index = 0_i32;
    unsafe {
        link.GetPath(&mut target, ptr::null_mut(), SLGP_RAWPATH.0 as _)
            .map_err(|error| {
                AppError::failure(format!("could not read Shell Link target: {error}"))
            })?;
        link.GetArguments(&mut arguments).map_err(|error| {
            AppError::failure(format!("could not read Shell Link arguments: {error}"))
        })?;
        link.GetWorkingDirectory(&mut working_directory)
            .map_err(|error| {
                AppError::failure(format!(
                    "could not read Shell Link working directory: {error}"
                ))
            })?;
        link.GetIconLocation(&mut icon, &mut icon_index)
            .map_err(|error| {
                AppError::failure(format!("could not read Shell Link icon: {error}"))
            })?;
        link.GetDescription(&mut description).map_err(|error| {
            AppError::failure(format!(
                "could not read Shell Link managed metadata: {error}"
            ))
        })?;
    }

    Ok(ShellLinkData {
        target: PathBuf::from(from_wide_buffer(&target)),
        arguments: from_wide_buffer(&arguments),
        root: PathBuf::from(from_wide_buffer(&working_directory)),
        icon: PathBuf::from(from_wide_buffer(&icon)),
        description: from_wide_buffer(&description),
    })
}

fn unique_temporary_path(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    for sequence in 0..100_u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| AppError::failure(format!("system clock error: {error}")))?
            .as_nanos();
        let path = root.join(format!(
            ".{name}.{}-{nonce}-{sequence}.tmp.lnk",
            std::process::id()
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(AppError::failure(format!(
        "could not allocate a temporary Windows launcher path in '{}'",
        root.display()
    )))
}

impl LauncherBackend for WindowsBackend {
    fn launcher_root(&self) -> &LauncherRoot {
        &self.root
    }

    fn inspect(&self, name: &str) -> Result<LauncherInspection, AppError> {
        let path = self.launcher_path(name);
        let _apartment = ComApartment::initialize()?;
        self.inspect_path(name, path)
    }

    fn enumerate(&self) -> Result<Vec<LauncherEnumerationItem>, AppError> {
        if !self.root.as_path().exists() {
            return Ok(Vec::new());
        }
        let entries = fs::read_dir(self.root.as_path()).map_err(|error| {
            AppError::failure(format!(
                "could not enumerate Windows launchers in '{}': {error}",
                self.root.as_path().display()
            ))
        })?;
        let _apartment = ComApartment::initialize()?;
        let mut launchers = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    launchers.push(Err(LauncherEnumerationError::from_io(
                        self.root.as_path().to_owned(),
                        "could not read directory entry",
                        error,
                    )));
                    continue;
                }
            };
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(OsStr::new("lnk")))
            {
                continue;
            }
            let Some(name) = path.file_stem().and_then(OsStr::to_str) else {
                continue;
            };
            match self.inspect_path(name, path.clone()) {
                Ok(LauncherInspection::Managed(launcher)) => launchers.push(Ok(launcher)),
                Ok(LauncherInspection::Missing | LauncherInspection::Foreign { .. }) => {
                    if read_shell_link(&path)
                        .map(|link| link.description == OsStr::new(FORMAT_DESCRIPTION))
                        .unwrap_or(false)
                    {
                        launchers.push(Err(LauncherEnumerationError::new(
                            path,
                            "Git Pin Windows Shell Link metadata is invalid",
                        )));
                    }
                }
                Err(error) => launchers.push(Err(LauncherEnumerationError::new(
                    path,
                    format!("could not inspect Windows Shell Link: {error}"),
                ))),
            }
        }
        Ok(launchers)
    }

    fn create(
        &self,
        repository: &Repository,
        ide_executable: &Path,
    ) -> Result<CreateOutcome, AppError> {
        let _apartment = ComApartment::initialize()?;
        fs::create_dir_all(self.root.as_path()).map_err(|error| {
            AppError::failure(format!(
                "could not create Windows launcher directory '{}': {error}",
                self.root.as_path().display()
            ))
        })?;
        let final_path = self.launcher_path(repository.name());
        if final_path.exists() {
            return Ok(CreateOutcome::Occupied(
                self.inspect_path(repository.name(), final_path)?,
            ));
        }

        let temporary = unique_temporary_path(self.root.as_path(), repository.name())?;
        let prepare_result = (|| -> Result<(), AppError> {
            create_shell_link(&temporary, repository, ide_executable)?;
            match self.inspect_path(repository.name(), temporary.clone())? {
                LauncherInspection::Managed(launcher)
                    if paths_equivalent(
                        &launcher.root,
                        repository.root(),
                        Platform::Windows,
                    ) => Ok(()),
                inspection => Err(AppError::failure(format!(
                    "temporary Windows Shell Link '{}' failed managed metadata validation: {inspection:?}",
                    temporary.display()
                ))),
            }
        })();
        if let Err(error) = prepare_result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }

        match fs::rename(&temporary, &final_path) {
            Ok(()) => match self.inspect_path(repository.name(), final_path.clone())? {
                LauncherInspection::Managed(launcher) => Ok(CreateOutcome::Created(launcher)),
                inspection => {
                    let _ = fs::remove_file(&final_path);
                    Err(AppError::failure(format!(
                        "committed Windows Shell Link '{}' failed managed metadata validation: {inspection:?}",
                        final_path.display()
                    )))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temporary);
                Ok(CreateOutcome::Occupied(
                    self.inspect_path(repository.name(), final_path)?,
                ))
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(AppError::failure(format!(
                    "could not atomically commit Windows Shell Link '{}': {error}",
                    final_path.display()
                )))
            }
        }
    }

    fn remove(&self, launcher: &ManagedLauncher) -> Result<(), AppError> {
        let _apartment = ComApartment::initialize()?;
        let expected_path = self.launcher_path(&launcher.name);
        if !paths_equivalent(&expected_path, &launcher.path, Platform::Windows) {
            return Err(AppError::failure(format!(
                "refusing to remove Windows Shell Link '{}' outside managed launcher root '{}'",
                launcher.path.display(),
                self.root.as_path().display()
            )));
        }
        match self.inspect_path(&launcher.name, launcher.path.clone())? {
            LauncherInspection::Missing => Ok(()),
            LauncherInspection::Managed(current)
                if current.name == launcher.name
                    && paths_equivalent(&current.path, &launcher.path, Platform::Windows)
                    && paths_equivalent(&current.root, &launcher.root, Platform::Windows) =>
            {
                fs::remove_file(&current.path).map_err(|error| {
                    AppError::failure(format!(
                        "could not remove Windows Shell Link '{}': {error}",
                        current.path.display()
                    ))
                })
            }
            _ => Err(AppError::failure(format!(
                "refusing to remove Windows Shell Link '{}' because its managed metadata changed",
                launcher.path.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        create_shell_link, quote_single_argument, read_shell_link, ComApartment, WindowsBackend,
        FORMAT_DESCRIPTION,
    };
    use crate::launcher::{CreateOutcome, LauncherBackend, LauncherInspection, ManagedLauncher};
    use crate::repo::{Platform, Repository};
    use crate::{
        app::{pin, unpin, PinOutcome, UnpinOutcome, UnpinTarget},
        repo::paths_equivalent,
    };
    use std::ffi::OsStr;
    use std::fs;
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
                "git-pin-windows-test-{}-{nonce}",
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

    #[test]
    fn uses_injected_launcher_root() {
        let temporary = TempDir::new();
        let launcher_root = temporary.0.join("launchers");
        let backend = WindowsBackend::for_test(launcher_root.clone());
        assert_eq!(backend.launcher_root().as_path(), launcher_root);
        assert_eq!(
            backend.launcher_path("project"),
            launcher_root.join("project.lnk")
        );
    }

    #[test]
    fn quotes_exactly_one_windows_command_line_argument() {
        assert_eq!(
            quote_single_argument(OsStr::new(r#"C:\work\项目 with "quotes"\"#)),
            OsStr::new(r#""C:\work\项目 with \"quotes\"\\""#)
        );
    }

    #[test]
    fn writes_and_reads_all_managed_shell_link_fields() {
        let _apartment = ComApartment::initialize().expect("COM must initialize");
        let temporary = TempDir::new();
        let ide = temporary.0.join("Cursor 编辑器").join("cursor.exe");
        fs::create_dir_all(ide.parent().expect("IDE fixture must have a parent"))
            .expect("IDE fixture directory must be created");
        fs::write(&ide, "fixture").expect("IDE fixture must be created");
        let root = temporary.0.join("项目 with spaces");
        fs::create_dir_all(&root).expect("repository fixture must be created");
        let repository = Repository::fixture(root.clone(), "项目 with spaces");
        let shortcut = temporary.0.join("project.lnk");

        create_shell_link(&shortcut, &repository, &ide).expect("Shell Link must be created");
        let fields = read_shell_link(&shortcut).expect("Shell Link must be readable");
        assert!(paths_equivalent(&fields.target, &ide, Platform::Windows));
        assert_eq!(fields.arguments, quote_single_argument(root.as_os_str()));
        assert!(paths_equivalent(&fields.root, &root, Platform::Windows));
        assert!(paths_equivalent(
            &fields.icon,
            &fields.target,
            Platform::Windows
        ));
        assert_eq!(fields.description, OsStr::new(FORMAT_DESCRIPTION));
    }

    #[test]
    fn backend_atomically_creates_inspects_and_safely_removes_shell_links() {
        let temporary = TempDir::new();
        let ide = temporary.0.join("Cursor 编辑器").join("cursor.exe");
        fs::create_dir_all(ide.parent().expect("IDE fixture must have a parent"))
            .expect("IDE fixture directory must be created");
        fs::write(&ide, "fixture").expect("IDE fixture must be created");
        let launcher_root = temporary.0.join("launchers");
        let backend = WindowsBackend::for_test(launcher_root.clone());
        let repository_root = temporary.0.join("项目 with spaces");
        fs::create_dir_all(&repository_root).expect("repository fixture must be created");
        let repository = Repository::fixture(repository_root, "项目 with spaces");

        let launcher = match backend
            .create(&repository, &ide)
            .expect("Shell Link creation must succeed")
        {
            CreateOutcome::Created(launcher) => launcher,
            CreateOutcome::Occupied(_) => panic!("isolated launcher slot must be empty"),
        };
        assert_eq!(launcher.path, launcher_root.join("项目 with spaces.lnk"));
        assert!(paths_equivalent(
            &launcher.ide_executable,
            &ide,
            Platform::Windows
        ));
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("Shell Link inspection must succeed"),
            LauncherInspection::Managed(launcher.clone())
        );
        assert!(!launcher_root
            .read_dir()
            .expect("launcher root must be readable")
            .any(|entry| entry
                .expect("directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .contains(".tmp.lnk")));

        backend
            .remove(&launcher)
            .expect("managed removal must succeed");
        assert!(!launcher.path.exists());
    }

    #[test]
    fn enumeration_matches_inspection_ignores_foreign_and_tolerates_missing_code() {
        let temporary = TempDir::new();
        let code = temporary.0.join("Code.exe");
        fs::write(&code, "fixture").expect("Code fixture must be created");
        let launcher_root = temporary.0.join("launchers");
        let backend = WindowsBackend::for_test(launcher_root.clone());
        let repository = Repository::fixture(temporary.0.join("project"), "project");
        assert!(backend
            .enumerate()
            .expect("missing root is empty")
            .is_empty());
        let launcher = match backend.create(&repository, &code).unwrap() {
            CreateOutcome::Created(launcher) => launcher,
            CreateOutcome::Occupied(_) => panic!("fixture must be empty"),
        };
        fs::write(launcher_root.join("foreign.lnk"), "foreign").unwrap();
        fs::remove_file(&code).unwrap();

        let enumerated = backend.enumerate().unwrap();
        assert_eq!(enumerated.len(), 1);
        assert_eq!(enumerated.into_iter().next().unwrap().unwrap(), launcher);
        assert!(matches!(
            backend.inspect("project").unwrap(),
            LauncherInspection::Managed(_)
        ));
    }

    #[test]
    fn backend_preserves_foreign_and_out_of_root_shell_links() {
        let temporary = TempDir::new();
        let code = temporary.0.join("Code.exe");
        fs::write(&code, "fixture").expect("Code fixture must be created");
        let launcher_root = temporary.0.join("launchers");
        fs::create_dir_all(&launcher_root).expect("launcher root must be created");
        let backend = WindowsBackend::for_test(launcher_root.clone());
        let foreign = launcher_root.join("project.lnk");
        fs::write(&foreign, "not a Shell Link").expect("foreign fixture must be created");

        assert_eq!(
            backend.inspect("project").expect("inspection must succeed"),
            LauncherInspection::Foreign {
                path: foreign.clone()
            }
        );
        let repository = Repository::fixture(temporary.0.join("project"), "project");
        assert_eq!(
            backend
                .create(&repository, &code)
                .expect("occupied create must be reported"),
            CreateOutcome::Occupied(LauncherInspection::Foreign {
                path: foreign.clone()
            })
        );
        assert!(foreign.exists());

        let outside = temporary.0.join("outside.lnk");
        let fake = ManagedLauncher {
            name: "project".to_owned(),
            root: temporary.0.join("project"),
            ide_executable: code.clone(),
            path: outside,
        };
        assert!(backend.remove(&fake).is_err());
    }

    #[test]
    fn isolated_shell_link_backend_integrates_with_shared_orchestration() {
        let temporary = TempDir::new();
        let code = temporary.0.join("Visual Studio Code").join("Code.exe");
        fs::create_dir_all(code.parent().expect("Code fixture must have a parent"))
            .expect("Code fixture directory must be created");
        fs::write(&code, "fixture").expect("Code fixture must be created");
        let backend = WindowsBackend::for_test(temporary.0.join("launchers"));
        let repository_root = temporary.0.join("工作 project with spaces");
        fs::create_dir_all(&repository_root).expect("repository fixture must be created");
        let repository = Repository::fixture(repository_root.clone(), "工作 project with spaces");

        let launcher = match pin(&backend, &repository, &code, Platform::Windows)
            .expect("first pin must create a Shell Link")
        {
            PinOutcome::Created(launcher) => launcher,
            PinOutcome::AlreadyPinned(_) => panic!("isolated slot must initially be empty"),
        };
        assert!(paths_equivalent(
            &launcher.root,
            &repository_root,
            Platform::Windows
        ));
        assert!(matches!(
            pin(&backend, &repository, &code, Platform::Windows)
                .expect("repeat pin must be idempotent"),
            PinOutcome::AlreadyPinned(_)
        ));

        let conflicting_root = temporary.0.join("other").join("工作 project with spaces");
        fs::create_dir_all(&conflicting_root).expect("conflict fixture must be created");
        let conflicting = Repository::fixture(conflicting_root, repository.name());
        let error = pin(&backend, &conflicting, &code, Platform::Windows)
            .expect_err("same-name different-root pin must conflict");
        assert!(error.to_string().contains("already points to"));

        assert!(matches!(
            unpin(
                &backend,
                &UnpinTarget::Repository(repository.clone()),
                Platform::Windows
            )
            .expect("matching repository must unpin"),
            UnpinOutcome::Removed(_)
        ));
        assert!(!launcher.path.exists());
        assert_eq!(
            backend
                .inspect(repository.name())
                .expect("removed launcher must inspect cleanly"),
            LauncherInspection::Missing
        );
    }
}
