# Git Pin

[简体中文](README_zh.md)

Git Pin provides the `git pin` and `git unpin` external commands for adding or
removing a Git repository from the native desktop application launcher. V1.2
supports configurable IDE command-line executables instead of binding launchers
to one editor product.

## Requirements

- Windows 10/11, a supported Linux desktop, or a currently supported macOS
  release.
- Git available on `PATH`.
- An IDE CLI that accepts one repository root as a positional argument:
  `ide path/to/repository`.
- No administrator/root permission is required. Git Pin only writes to the
  current user's launcher directories.

On macOS, generated application bundles are unsigned. The first launch can
therefore show the normal Gatekeeper warning for locally generated unsigned
software. Git Pin does not modify `/Applications` and does not download code.

## Portable installation

V1 publishes runner-native x86_64 packages for Windows, Linux, and macOS:

- `git-pin-v<version>-windows-x86_64.zip`
- `git-pin-v<version>-linux-x86_64.zip`
- `git-pin-v<version>-macos-x86_64.zip`

Arm64 packages are not part of the V1 support matrix because their complete
native build, test, packaging, and launcher behavior has not yet passed the
release gate on every advertised platform.

1. Download the x86_64 ZIP matching the operating system from the GitHub
   Release.
2. Extract the ZIP. Its top-level directory contains `git-pin`, `git-unpin`,
   `README.md`, and `LICENSE` (`.exe` is present on both Windows binaries).
3. Add that extracted top-level directory to the user `PATH`.
4. Run `git pin -h` or `git-pin --help` to confirm the command is available.

Git reserves `git pin --help` for its own documentation lookup before it
dispatches the external `git-pin` executable. Use `git pin -h` or direct
`git-pin --help` for Git Pin's complete help.

The package is portable: it includes no installer, does not edit the registry,
and does not modify `PATH` automatically.

## IDE configuration

Git Pin reads `pin.ide` through Git's configuration interface. Its default is
`code`. Set an executable name available on `PATH`:

```text
git config --global pin.ide cursor
```

Or set one executable path, including a path containing spaces:

```text
git config --global pin.ide "/opt/Custom IDE/bin/custom-ide"
```

Repository configuration can override the global value:

```text
git config pin.ide zed
```

One invocation can override all persisted scopes:

```text
git -c pin.ide=cursor pin
```

Git's normal command-line, repository, global, and system precedence applies,
including Git includes and conditional includes. `pin.ide` is one atomic
executable name or path—not a shell command, argument list, placeholder, or
command template. The selected CLI must implement the `ide path/to/repository`
contract without requiring extra arguments.

Git Pin resolves the executable to an absolute path when pinning and freezes
that path into the new launcher. Later configuration or `PATH` changes do not
rewrite existing launchers. To switch an existing launcher, remove and recreate
it:

```text
git unpin path/to/repository
git pin path/to/repository
```

## Usage

From a repository or any directory inside its working tree:

```text
git pin
git unpin
```

With an explicit repository or subdirectory path:

```text
git pin path/to/repository
git unpin path/to/repository
```

When the repository no longer exists, remove its entry by exact basename:

```text
git unpin repository-name
```

Inspect every Git Pin managed launcher, or remove all stale launchers:

```text
git pin --list
git pin --prune
```

`--list` prints each repository name, its recorded root, and either `valid` or
`invalid` with a reason. A root is valid only when it exists, is a directory,
belongs to a Git working tree, and is that working tree's top-level root. List
is read-only; an empty list is a successful, explicit result.

`--prune` rechecks and removes only recognized Git Pin launchers whose recorded
roots are missing, not directories, no longer Git working trees, or no longer
match Git's top-level root. Valid launchers and unrecognized files or apps are
preserved. A frozen IDE executable being moved or deleted is neither an invalid
repository status nor a prune condition. Repeating prune when no stale entries
remain succeeds. If one entry cannot be read, checked, or removed, processing
continues for the other entries and the command ultimately returns a non-zero
status with all available diagnostics.

Git determines the top-level working tree. The launcher's display name is the
root directory basename and is not silently rewritten. Repeating `git pin` for
the same root is successful and preserves the existing launcher and frozen IDE.
If another root has the same basename, Git Pin reports the existing target and
refuses to overwrite it. Removing an absent entry is also successful.

## Launcher locations

- Windows: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<name>.lnk`
- Linux: `${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<name>.desktop`
- macOS: `$HOME/Applications/Git Pin/<name>.app`

The launcher itself is the V1 registry. Git Pin reads platform-native metadata
before replacing or deleting anything and refuses to remove unrecognized
artifacts. Launchers created by v1.0/v1.1 remain inspectable and removable.

## V1.2 scope

`git pin` accepts zero or one positional argument, or exactly one of `--help`,
`-h`, `--list`, and `--prune`. `git unpin` accepts zero or one positional
argument. V1.2 does not provide extra IDE arguments, shell templates, `--name`,
`--all`, JSON/filter output, a separate metadata database, automatic
installation, automatic updates, forced launcher refresh, or automatic `PATH`
modification. It does not guarantee immediate refresh of every third-party
desktop launcher cache.

## Development

GitHub Actions on Windows, Linux, and macOS are the authoritative compilation,
test, packaging, and compatibility gates. Contributors with Rust installed can
run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test --locked --all-targets` locally; local results do not replace the
required CI checks.

## Acknowledgements

Thanks to the [LINUX DO](https://linux.do/) community for the support, feedback, and encouragement during the development and sharing of GitPin.

## License

Git Pin is distributed under the MIT License. See `LICENSE`.
