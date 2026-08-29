# Git Pin

[简体中文](README_zh.md)

Git Pin provides the `git pin` and `git unpin` external commands for adding or
removing a Git repository from the native desktop application launcher.

## Requirements

- Windows 10/11, a supported Linux desktop, or a currently supported macOS
  release.
- Git available on `PATH`.
- The stable release of Visual Studio Code. V1 does not discover VS Code
  Insiders, VSCodium, or arbitrary custom installations.
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
4. Confirm Git can dispatch the external commands with `git pin --help`. The
   complete help is printed to standard output with exit status 0.

The package is portable: it includes no installer, does not edit the registry,
and does not modify `PATH` automatically.

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
preserved. Visual Studio Code being unavailable is neither an invalid status
nor a prune condition. Repeating prune when no stale entries remain succeeds.
If one entry cannot be read, checked, or removed, processing continues for the
other entries and the command ultimately returns a non-zero status with all
available diagnostics.

Git determines the top-level working tree. The launcher's display name is the
root directory basename and is not silently rewritten. Repeating `git pin` for
the same root is successful and leaves one entry. If another root has the same
basename, Git Pin reports the existing target and refuses to overwrite it.
Removing an absent entry is also successful.

## Launcher locations

- Windows: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<name>.lnk`
- Linux: `${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<name>.desktop`
- macOS: `$HOME/Applications/Git Pin/<name>.app`

The launcher itself is the V1 registry. Git Pin reads platform-native metadata
before replacing or deleting anything and refuses to remove unrecognized
artifacts.

## V1.1 scope

`git pin` accepts zero or one positional argument, or exactly one of `--help`,
`-h`, `--list`, and `--prune`. `git unpin` accepts zero or one positional
argument. V1.1 does not provide `--name`, `--all`, JSON/filter output,
configuration files, a separate metadata database, automatic installation,
automatic updates, VS Code channel selection, or automatic `PATH` modification.
It does not guarantee immediate refresh of every third-party desktop launcher
cache.

## Development

A local Rust toolchain is not required. Push changes to a branch and use the
GitHub Actions results as the authoritative compilation and test validation
before merging. Contributors with Rust installed may run the same checks
locally, but local results do not replace the required CI checks.

## License

Git Pin is distributed under the MIT License. See `LICENSE`.
