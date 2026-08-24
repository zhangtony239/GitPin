# Git Pin

[English](README.md)

Git Pin 提供 `git pin` 与 `git unpin` 两个 Git 外部命令，用于将 Git 仓库添加到操作系统的原生桌面应用启动器，或从中移除。

## 前置条件

- Windows 10/11、受支持的 Linux 桌面系统，或当前仍受支持的 macOS 版本。
- `PATH` 中可以运行 Git。
- 已安装稳定版 Visual Studio Code。V1 不查找 VS Code Insiders、VSCodium 或任意自定义安装位置。
- 不需要管理员或 root 权限。Git Pin 只写入当前用户的启动器目录。

在 macOS 上，生成的应用 bundle 未签名，因此首次启动时可能出现针对本地生成未签名软件的常规 Gatekeeper 提示。Git Pin 不修改 `/Applications`，也不下载代码。

## Portable 安装

V1 为 Windows、Linux 和 macOS 发布 runner-native x86_64 包：

- `git-pin-v<version>-windows-x86_64.zip`
- `git-pin-v<version>-linux-x86_64.zip`
- `git-pin-v<version>-macos-x86_64.zip`

arm64 包不属于 V1 支持矩阵，因为其完整原生构建、测试、打包和启动器行为尚未在每个宣称平台上通过 release 门禁。

1. 从 GitHub Release 下载与操作系统匹配的 x86_64 ZIP。
2. 解压 ZIP。其顶层目录包含 `git-pin`、`git-unpin`、`README.md` 和 `LICENSE`；Windows 上两个 binary 均带 `.exe` 后缀。
3. 将解压后的顶层目录加入用户 `PATH`。
4. 运行 `git pin --help`，确认 Git 可以分派外部命令。V1 会有意拒绝选项，因此出现 usage 信息并返回状态码 2 即表示分派正常。

发布包是 portable 的：不包含安装器、不编辑 registry，也不会自动修改 `PATH`。

## 用法

在仓库或其工作树中的任意目录运行：

```text
git pin
git unpin
```

传入明确的仓库或子目录路径：

```text
git pin path/to/repository
git unpin path/to/repository
```

仓库已被删除时，可以按精确 basename 移除入口：

```text
git unpin repository-name
```

Git Pin 以 Git 认定的顶层工作树为准。启动器显示名称取自根目录 basename，且不会被静默改写。对同一个 root 重复运行 `git pin` 会成功，并保持只有一个入口。如果另一个 root 具有相同 basename，Git Pin 会报告现有目标并拒绝覆盖。移除不存在的入口也会成功。

## 启动器位置

- Windows：`%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<name>.lnk`
- Linux：`${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<name>.desktop`
- macOS：`$HOME/Applications/Git Pin/<name>.app`

平台启动器本身就是 V1 registry。Git Pin 会在替换或删除任何内容之前读取平台原生 metadata，并拒绝删除无法识别的产物。

## V1 范围

V1 有意只接受零个或一个位置参数。它不提供 `--name`、`--list`、`--prune`、`--all`、配置文件、独立 metadata database、自动安装、自动更新、VS Code 渠道选择或自动修改 `PATH`。它也不保证每个第三方桌面启动器缓存都会立即刷新。

## 开发

本地不要求安装 Rust toolchain。将变更推送到分支后，以 GitHub Actions 结果作为合并前权威的编译和测试验证。已安装 Rust 的贡献者可以在本地执行相同检查，但本地结果不能替代必需的 CI 检查。

## 许可证

Git Pin 使用 MIT License 分发，详见 `LICENSE`。
