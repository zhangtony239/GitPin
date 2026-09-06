## Context

见 `proposal.md` 的动机。v1.2 的 pin 流程先用 Git 读取 `pin.ide`，未设置时返回字符串 `code`，再由通用解析器按当前平台的 PATH 规则把名称转换为绝对路径。Windows 实现依次尝试 `PATHEXT` 候选；标准 VS Code 安装加入 PATH 的是 `Microsoft VS Code\bin`，因此通常命中 `code.cmd`。Windows backend 随后将解析结果原样写入 Shell Link 的 TargetPath。

v1.0/v1.1 的 Windows 行为不同：平台发现逻辑最终选择 GUI `Code.exe`，所以开始菜单入口不会启动可见命令窗口。v1.2 又要求 launcher 固化创建时选定的 executable，且重复 pin 不隐式改写既有 launcher。修复必须同时遵守这些约束，行为要求见本变更的 `repository-pinning` delta spec。

## Goals / Non-Goals

**Goals:**

- 让 Windows 默认 `code` 选择产生适合桌面 Shell Link 的 GUI `Code.exe` 绝对路径。
- 保持通用 IDE executable 解析与 Windows GUI 默认值适配之间的职责清晰，避免影响 Linux/macOS。
- 以真实 VS Code 常见目录形状覆盖回归，并验证最终 `.lnk` 的 TargetPath、参数、工作目录和图标。
- 保持错误路径安全：找不到可验证 GUI 目标时不退回固化 `.cmd`。

**Non-Goals:**

- 不自动更新 v1.2 已创建的 `code.cmd` launcher，也不增加 `--refresh` 或 migration 子命令。
- 不为 Cursor、Insiders、VSCodium 或任意第三方 CLI wrapper 推断 GUI executable。
- 不改变显式 `pin.ide` 值作为单个 executable 的配置模型，也不增加 IDE 参数模板。
- 不改变 Linux Desktop Entry 或 macOS bundle 的启动策略。

## Decisions

### 1. 保留“是否使用默认值”的来源信息

应用层不能仅凭最终字符串等于 `code` 判断默认行为，因为用户可能显式设置 `pin.ide=code` 并期待现有的原子 executable 语义。配置读取结果应保留“配置缺失而采用默认值”与“用户显式配置”的区别，随后只为前者启用 Windows 默认 VS Code GUI 解析。

这比比较字符串可靠，也符合本次“修好默认逻辑”的范围。另一方案是所有值为 `code` 的调用均走 GUI 映射，但它会悄然改变显式配置的含义，并让配置来源不可诊断，因此不采用。

### 2. Windows 默认解析先定位 CLI shim，再验证相邻 GUI executable

默认 Windows 解析继续使用 PATH 找到用户当前可调用的稳定版 `code`。若结果是标准布局中的 `...\Microsoft VS Code\bin\code.cmd`，则从其父级安装目录构造 `Code.exe` 候选，并要求候选存在、是文件、扩展名为可执行程序且可规范化为绝对路径。只有验证成功的 `Code.exe` 才可进入 launcher backend。

该方式继承用户 PATH 所选择的安装实例，避免重新维护一整套用户级/系统级安装目录优先级，同时恢复 v1.0 的 GUI 目标。若 PATH 直接提供可接受的 GUI `code.exe`，可以按验证后的 executable 使用；若只发现无法识别布局的脚本 shim，则明确失败。

替代方案是扫描 `%LOCALAPPDATA%`、`ProgramFiles` 等固定安装目录。它能绕过 PATH，但会复制安装发现优先级，容易选中与终端 `code` 不同的实例，并扩大 portable/自定义安装兼容范围，因此本次不采用。解析 `code.cmd` 文件内容也不采用，因为批处理内容属于上游实现细节，解析与安全边界更复杂。

### 3. Shell Link backend 继续只接受最终可固化路径

Windows backend 不负责识别 `code`、解释 PATH 或把 `.cmd` 映射为 GUI 程序；它继续接收上游已解析的绝对路径并写入 TargetPath、IconLocation、参数及工作目录。默认适配应发生在配置/应用解析边界，保证 backend 契约仍适用于任意显式 IDE executable。

这样可以避免在创建、inspect 和 enumerate 三条路径中重复品牌逻辑，也使现有 launcher 仍能被读取和删除。另一方案是在 `create` 时特判 `code.cmd`，但 inspect 得到的是固化路径而不是配置来源，且 backend 无法区分默认值与显式值，因此不采用。

### 4. 对脚本型默认入口失败关闭

默认 `code` 命中 `.cmd`/`.bat` 后，若无法验证对应 GUI executable，pin 返回包含默认值、shim 路径和 GUI 候选上下文的错误。系统不得直接固化脚本，也不得调用 `cmd.exe /c` 作为替代。

失败关闭会让非标准的自定义 `code.cmd` 默认入口在升级后需要显式配置实际 executable，但能保证新 launcher 满足“无可见控制台窗口”的 v1.2.1 契约。隐藏 `cmd.exe` 窗口的代理方案仍引入 shell、参数转发及生命周期问题，与当前架构不符。

### 5. 分层测试锁定配置来源与原生 Shell Link 结果

通用测试覆盖配置缺失与显式 `pin.ide=code` 的来源差异；Windows 解析测试创建 `install\bin\code.cmd` 与 `install\Code.exe` fixture，验证默认结果为后者，并覆盖缺失 GUI、直接 `.exe`、空格及非 ASCII 路径。Windows 原生测试用解析出的 GUI 路径创建 Shell Link，再读回并断言 TargetPath 不是 `.cmd`/`.bat`，且 root 仍是单一正确参数。

测试不以“观察到窗口闪烁”作为 CI 断言，因为窗口可见性依赖交互桌面；TargetPath 为 GUI executable 且不经过批处理解释器，是稳定、可自动验证的代理条件。

## Risks / Trade-offs

- **[VS Code 改变 `bin\code.cmd` 与 `Code.exe` 的相对布局]** → 将布局识别限制在已验证候选并提供具体错误；后续可按真实上游变化独立扩展候选，而不回退脚本目标。
- **[用户显式设置 `pin.ide=code` 时仍可能得到 `code.cmd`]** → 本修复有意只改变配置缺失的默认路径；文档说明若要稳定 GUI launcher，应不设置该键或显式配置 `Code.exe` 绝对路径。
- **[现有配置 API 返回字符串，增加来源信息会传播到调用测试]** → 使用小型类型表达配置选择来源，保持 executable 解析与 launcher trait 的路径契约不变，限制改动范围。
- **[既有 v1.2 launcher 继续弹出窗口]** → 遵守已发布的幂等语义，不静默改写；README 给出 `git unpin <repo> && git pin <repo>` 的显式修复步骤。
- **[仅验证 TargetPath 无法百分百证明所有机器不显示窗口]** → 使用 Windows GUI `.exe` 目标作为确定性契约，并在发布前进行一次交互式 smoke test 作为补充。

## Migration Plan

1. 扩展配置选择模型，使调用方知道 `code` 是否来自未配置时的默认值，并保持非 Pin 操作不读取 IDE 配置。
2. 加入 Windows 默认 VS Code GUI 解析及隔离 fixture 测试，再将结果接入现有 pin 编排。
3. 用 Windows 原生 Shell Link 测试验证最终目标、参数、工作目录、图标和既有 launcher inspect/unpin 兼容性。
4. 更新版本为 `1.2.1` 及中英文文档，执行格式化、三平台测试、clippy、OpenSpec 严格校验和 Windows 交互式 smoke test 后发布。
5. 回滚时可恢复 v1.2 binary；v1.2.1 新建的 `Code.exe` launcher 与旧版本格式兼容，无需磁盘迁移。已受影响用户按文档显式 unpin 后重新 pin。
