## Why

v1.2 在 Windows 上把默认 `code` 按 `PATH`/`PATHEXT` 解析为 VS Code 的 `code.cmd`，并将该批处理入口直接固化为开始菜单快捷方式目标，导致用户点击已 pin repository 时出现额外的命令提示符窗口。v1.2.1 需要恢复 v1.0/v1.1 的无控制台 GUI 启动体验，同时保留 v1.2 的可配置 IDE 能力。

## What Changes

- Windows 未配置 `pin.ide`、使用默认 `code` 时，将 VS Code CLI shim 解析到对应的 GUI `Code.exe`，而不是把 `code.cmd` 直接写入 `.lnk`。
- 保持显式配置的 IDE executable 语义不变；本次不建立其他 IDE CLI shim 到 GUI executable 的品牌映射。
- 对无法从默认 `code` 找到有效 GUI executable 的情况给出可诊断错误，不创建退化为命令窗口启动的快捷方式。
- 增加贴近 Windows VS Code 实际安装布局的解析和 Shell Link 测试，防止默认入口再次回归为 `.cmd`。
- 将 crate/package 版本更新为 `1.2.1`，并在中英文用户文档中记录该兼容性修复及既有 launcher 的重新创建方式。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `repository-pinning`: 收紧 Windows 默认 VS Code 启动器的可观察行为，要求开始菜单入口直接使用 GUI executable，启动 repository 时不得附带可见控制台窗口。

## Impact

- 主要影响 IDE 解析逻辑与 Windows Shell Link backend，以及相应的 Windows 单元/集成测试。
- 影响 `Cargo.toml`/lockfile 中的发布版本及 README 发布说明，不新增运行时依赖或公开 CLI/config 键。
- 仅影响 v1.2.1 以后新创建的 Windows 默认 Code launcher；遵循现有幂等规则，不自动改写已经固化为 `code.cmd` 的快捷方式，用户需先 unpin 再 pin 才能更新。
- Linux、macOS 和显式配置非默认 IDE executable 的行为不变。
