# Git Pin

[English](README.md)

Git Pin 提供 `git pin` 与 `git unpin` 两个 Git 外部命令，用于将 Git 仓库添加到操作系统的原生桌面应用启动器，或从中移除。V1.2 支持配置 IDE 命令行 executable，不再把启动器绑定到某个编辑器产品。

## 前置条件

- Windows 10/11、受支持的 Linux 桌面系统，或当前仍受支持的 macOS 版本。
- `PATH` 中可以运行 Git。
- IDE CLI 能以一个 repository root 位置参数启动：`ide path/to/repository`。
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
4. 运行 `git pin -h` 或 `git-pin --help`，确认命令可用。

Git 会在分派外部 `git-pin` executable 之前，将 `git pin --help` 占用为自身的文档查询。请使用 `git pin -h` 或直接运行 `git-pin --help` 查看 Git Pin 完整帮助。

发布包是 portable 的：不包含安装器、不编辑 registry，也不会自动修改 `PATH`。

## IDE 配置

Git Pin 通过 Git 配置接口读取 `pin.ide`，默认值是 `code`。可以设置为 `PATH` 中的 executable 名称：

```text
git config --global pin.ide cursor
```

也可以设置为单个 executable 路径，包括含空格路径：

```text
git config --global pin.ide "/opt/Custom IDE/bin/custom-ide"
```

仓库配置可以覆盖全局值：

```text
git config pin.ide zed
```

单次调用可以覆盖所有持久作用域：

```text
git -c pin.ide=cursor pin
```

配置遵循 Git 正常的命令行、仓库、全局和系统优先级，包括 Git include 与 conditional include。`pin.ide` 是一个原子的 executable 名称或路径，不是 shell 命令、参数列表、占位符或命令模板。所选 CLI 必须无需额外参数即可满足 `ide path/to/repository` 契约。

Git Pin 在 pin 时把 executable 解析为绝对路径，并固化到新启动器中。以后修改配置或 `PATH` 不会改写既有启动器。要切换既有入口，请先删除再重新创建：

```text
git unpin path/to/repository
git pin path/to/repository
```

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

检查全部 Git Pin 受管启动器，或清理全部僵尸启动器：

```text
git pin --list
git pin --prune
```

`--list` 输出每个 repository name、记录的 root，以及 `valid` 或带原因的 `invalid` 状态。只有 root 存在、是目录、属于 Git 工作树且正好是该工作树的顶层目录时才有效。list 是只读操作；空列表也是明确且成功的结果。

`--prune` 会在删除前复检，并且只删除能够识别为 Git Pin 管理、且记录 root 已不存在、不是目录、不再是 Git 工作树或不再匹配 Git 顶层目录的启动器。有效启动器以及无法识别的外部文件或应用都会保留。已固化的 IDE executable 后来被移动或删除，既不会使 repository root 失效，也不会触发清理。没有僵尸项时重复运行 prune 仍会成功。若某一项无法读取、自检或删除，命令会继续处理其他项，最终通过非零状态和汇总诊断报告失败。

Git Pin 以 Git 认定的顶层工作树为准。启动器显示名称取自根目录 basename，且不会被静默改写。对同一个 root 重复运行 `git pin` 会成功，并保留既有启动器及其固化 IDE。如果另一个 root 具有相同 basename，Git Pin 会报告现有目标并拒绝覆盖。移除不存在的入口也会成功。

## 启动器位置

- Windows：`%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<name>.lnk`
- Linux：`${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<name>.desktop`
- macOS：`$HOME/Applications/Git Pin/<name>.app`

平台启动器本身就是 V1 registry。Git Pin 会在替换或删除任何内容之前读取平台原生 metadata，并拒绝删除无法识别的产物。v1.0/v1.1 创建的启动器仍可被检查和移除。

## V1.2 范围

`git pin` 接受零个或一个位置参数，或单独接受 `--help`、`-h`、`--list`、`--prune` 之一；`git unpin` 接受零个或一个位置参数。V1.2 不提供额外 IDE 参数、shell 模板、`--name`、`--all`、JSON/过滤输出、独立 metadata database、自动安装、自动更新、强制刷新启动器或自动修改 `PATH`。它也不保证每个第三方桌面启动器缓存都会立即刷新。

## 开发

Windows、Linux 和 macOS 上的 GitHub Actions 是权威的编译、测试、打包与兼容门禁。已安装 Rust 的贡献者可以在本地执行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 与 `cargo test --locked --all-targets`；本地结果不能替代必需的 CI 检查。

## 致谢

感谢 [LINUX DO](https://linux.do/) 社区在 GitPin 的开发和分享过程中给予的支持、反馈和鼓励。

## 许可证

Git Pin 使用 MIT License 分发，详见 `LICENSE`。
