## MODIFIED Requirements

### Requirement: Windows 启动器行为
在 Windows 上，系统 SHALL 将受管启动器保存为 `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<repo-name>.lnk`。快捷方式 SHALL 直接启动 pin 时解析并固化的 IDE executable，以规范 root 作为单个参数和工作目录，并使用该 executable 可用的图标。未配置 `pin.ide`、因而选择默认 `code` 时，快捷方式 MUST 以 Visual Studio Code GUI executable 为目标，不得把 `.cmd` 或 `.bat` CLI shim 固化为目标，且从开始菜单启动 repository 时不得出现伴随的可见控制台窗口。路径比较 SHALL 遵循 Windows 的不区分大小写语义。

#### Scenario: 创建 Windows 快捷方式
- **WHEN** 用户在 Windows 上以有效 `pin.ide` 成功 pin 名为 `foo` 的 repository
- **THEN** `.pinned_repo\foo.lnk` 存在，且其目标、参数和工作目录可使开始菜单入口在所选 IDE 中打开规范 root

#### Scenario: Windows 路径包含空格或非 ASCII 字符
- **WHEN** IDE executable 或 repository root 包含空格、引号可表示字符或非 ASCII 字符
- **THEN** 快捷方式保留准确路径，启动时将整个 root 作为一个参数传递且不执行路径中的内容

#### Scenario: Windows 默认 Code 使用 GUI 入口
- **WHEN** Windows 用户未配置 `pin.ide`，且 PATH 中的 `code` 命令由 Visual Studio Code 的 `code.cmd` CLI shim 提供
- **THEN** 系统创建以对应有效 `Code.exe` 为目标的快捷方式，点击该入口在 Visual Studio Code 中打开规范 root，且不显示伴随的命令提示符窗口

#### Scenario: Windows 默认 Code 缺少有效 GUI 入口
- **WHEN** Windows 用户未配置 `pin.ide`，系统能发现 `code.cmd` 但无法定位或验证其对应的 Visual Studio Code GUI executable
- **THEN** pin 明确失败并报告默认 IDE 解析错误，且不创建以批处理 shim 为目标的快捷方式

#### Scenario: 既有批处理目标不被重复 pin 暗中更新
- **WHEN** 某 repository 的既有受管快捷方式已指向 `code.cmd` 且用户再次 pin 同一规范 root
- **THEN** 命令遵循既有幂等规则保持快捷方式不变，用户需先 unpin 再 pin 才能获得 v1.2.1 的 GUI 目标
