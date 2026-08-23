## Why

`git-pin` 目前只有项目备忘录，尚无可实施、可验证的 V1 契约。首个变更需要把核心命令、三平台启动器和可移植发布流程一次规划清楚，让实现可以按小步提交推进，并在无需本地 Rust 工具链的前提下完全由 CI 验证。

## What Changes

- 初始化 Rust CLI 项目，提供独立的 `git-pin` 与 `git-unpin` Git 外部命令。
- 支持从当前目录或给定路径发现并规范化 Git repository root，并以 root basename 作为入口名称。
- 定义 pin、按路径 unpin、按名称 unpin、幂等操作、同名仓库冲突和可诊断错误行为。
- 在 Windows 创建开始菜单 `.lnk`，在 Linux 创建 XDG `.desktop`，在 macOS 创建用户 Applications 下可被 Spotlight/Finder 启动的 `.app` bundle；三者均打开 VS Code 中的目标仓库。
- 不引入配置文件、元数据数据库、安装器或复杂参数系统；平台启动器本身同时充当 V1 registry。
- 建立 GitHub Actions 验证与发布：格式、lint、测试和 release build 均提交 CI，在原生 runner matrix 上生成各平台/架构 portable ZIP。
- 将实现拆成可独立由 CI 验证的小步提交；不要求贡献者本地安装 Rust 编译链。

## Capabilities

### New Capabilities

- `repository-pinning`: 定义 repository 发现、命名、pin/unpin 命令、冲突语义，以及 Windows、Linux、macOS 启动器的外部可观察行为。
- `portable-distribution`: 定义 CI 验证门禁、原生平台构建矩阵、portable ZIP 内容与发布命名。

### Modified Capabilities

无。

## Impact

- 新增 Rust workspace/crate、两个 binary entry point、共享核心逻辑及三套条件编译的平台 backend。
- Windows 依赖 Win32/COM API，Linux 遵循 XDG desktop entry 约定，macOS 生成标准应用 bundle；运行时仍需系统已安装 Git 与 Visual Studio Code。
- 新增 GitHub Actions CI/release workflow、README、LICENSE 及发布打包逻辑。
- 用户可见接口固定为 `git pin [path]` 与 `git unpin [path|name]`；V1 不承诺 `--name`、`--list`、`--prune`、`--all` 等扩展参数。
