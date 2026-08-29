## Context

当前应用层在 pin 流程中调用平台函数发现 Visual Studio Code，随后把固定 IDE 假设传给 launcher backend。Windows launcher 直接记录 Code executable，Linux Desktop Entry 生成 Code 命令，macOS bundle 的内部入口则通过系统 `open -a "Visual Studio Code"` 启动。受管元数据主要围绕 repository 名称与 root，list/prune 的有效性检查不依赖 IDE。

Git Pin 已依赖 Git 子进程完成 repository root 发现，因而可以复用 Git 本身读取配置，而无需引入第二套配置文件和优先级实现。另一个约束来自 Git 分派器：`git <external> --help` 在执行外部程序前被重写为 `git help <external>`，Rust CLI 无法截获该参数。

## Goals / Non-Goals

**Goals:**

- 让应用层得到一个经过 Git 配置优先级解析、且能安全作为单个程序启动的 IDE executable。
- 让三个平台把同一 `executable + repository root` 契约固化进 launcher，消除平台层的 Code 专用发现和名称假设。
- 保持 launcher 自包含：从桌面启动时不需要定位 repository 配置，也不因以后修改 `pin.ide` 改变行为。
- 用真实 Git 分派测试区分 `git pin -h` 与 Git 占用的 `git pin --help`。

**Non-Goals:**

- 不支持附加 IDE 参数、占位符、shell 命令模板或按 IDE 品牌维护适配表。
- 不动态更新既有 launcher，也不新增 migrate、refresh 或 repin 强制覆盖模式。
- 不替 Git 安装 `git-pin.html`，不修改 Git 的 help 配置或安装目录。
- list/prune 不因已固化 IDE 后来被移动或删除而将 repository launcher 判为僵尸。

## Decisions

### 1. 通过 Git 子进程读取单个有效配置值

应用在完成 CLI 动作分派后，只有 Pin 动作读取 `pin.ide`。读取应调用 Git 配置接口获取本次进程上下文的最终值，并保留 Git 原生的命令行、仓库、全局、系统优先级；未设置时由应用回退到 `code`。Help、List、Prune 与 Unpin 不需要 IDE，因此不得因配置损坏或 IDE 缺失失败。

选择复用 Git 而不是自行解析各级 config 文件，是为了正确处理 include、条件 include、平台路径、命令行 `-c` 传递及未来 Git 行为。配置读取错误需要包含键名与 Git 诊断上下文。

### 2. `pin.ide` 是原子 executable 值，不是命令行

配置值整体表示 PATH 命令名或文件路径。解析层不进行 shell 拆词，也不接受可执行文件后附参数。若值包含空格，先按完整路径处理；仅当它是无目录组件的名称时才按 PATH 规则搜索。解析结果规范化为绝对 executable 路径，再交给 backend 固化。

这允许 `code`、`cursor`、`zed` 或自定义 CLI wrapper，同时避免引入跨平台 quoting 语言与命令注入面。额外参数未来若需要，应设计结构化配置键，而不是扩展当前字符串为 shell 模板。

### 3. IDE executable 成为 launcher 创建输入和受管元数据的一部分

共享 launcher 创建契约从仅接收 repository 扩展为接收已解析 IDE executable。Windows `.lnk` TargetPath、Linux Desktop Entry `Exec`、macOS bundle 内部启动入口都直接调用该 executable，并把 root 作为恰好一个参数。

inspect/enumerate 应能从现有平台 launcher 恢复已固化 executable，以便测试、诊断和一致性校验，但 repository 有效性仍只由 root 决定。v1.0/v1.1 的 Code launcher 天然可解释为固化了 Code，不需要磁盘迁移。

相比 launcher 启动时调用 Git 读取配置，固化设计不依赖启动时工作目录、仓库仍存在或 Git 配置上下文，且符合用户确认的“只影响以后创建”的语义。

### 4. 重复 pin 保持幂等，不借配置变化隐式更新

现有同名且 root 相同的 launcher 继续返回 already pinned，即使当前 `pin.ide` 与已固化 executable 不同。这样避免一次看似幂等的 pin 悄悄改变用户桌面入口。要切换既有入口，v1.2 使用显式的 unpin 后重新 pin 流程，并在 README 中说明。

未来若频繁切换成为需求，可单独设计 `--refresh`，同时定义原子替换和失败恢复；本次不提前加入。

### 5. macOS 改为直接执行 CLI，而非应用名映射

macOS 内嵌 launcher 读取 bundle 中固化的 IDE executable 与 repository root，并以无 shell的进程 API直接执行 `ide root`。不再使用 `open -a "Visual Studio Code" --args root`，因为任意 CLI 路径无法可靠映射为 `.app` 显示名，且用户明确要求兼容任何 PATH CLI。

bundle 继续复制 `git-pin` 机器码作为内部入口，但 metadata 必须同时携带 IDE 路径。解析时对该字段执行与其他受管字段相同的完整性校验。

### 6. 帮助契约遵循 Git 的不可覆盖分派规则

二进制继续接受 `--help` 与 `-h`，因此 `git-pin --help`、`git-pin -h` 和 `git pin -h` 都显示相同文本。帮助中显式提醒 `git pin --help` 由 Git 自己解释为文档查询，并列出两个可靠入口。

跨进程测试必须实际运行 `git pin -h`，不能只直接执行测试二进制；对于 `git pin --help`，测试验证外部二进制未被调用或记录 Git 文档失败特征会过度绑定 Git 安装，因此以文档契约检查和 Git 行为说明为主。

## Risks / Trade-offs

- **[不同平台对 PATH executable 的可执行性判断不同]** → 集中复用现有 Code 查找逻辑的安全部分，输出绝对路径，并为名称、绝对路径、含空格路径、缺失和目录值建立三平台测试。
- **[Git `-c pin.ide=... pin` 的配置能否传到外部命令]** → 通过真实 Git 分派集成测试验证；读取必须使用继承到外部命令的 Git 配置参数环境，而非启动一个丢失该上下文的无关解析流程。
- **[旧 launcher 缺少显式 IDE metadata]** → 平台解析首先从真实启动目标恢复 executable；新增 metadata 只在无法无歧义恢复的平台使用，并保持旧 Code launcher 可识别。
- **[配置值是相对路径时语义不稳定]** → 含目录组件的路径按 pin 进程当前目录解析并立即绝对化；无目录组件的值只走 PATH 搜索，固化后不依赖未来 PATH。
- **[IDE CLI 接受 root 参数的方式不一致]** → v1.2 只承诺兼容 `ide path/to/repo` 契约，不内置品牌适配；不符合该 CLI 契约的 IDE 不在支持范围。
- **[配置改变但重复 pin 不更新会令用户意外]** → README 明确固化规则和 `git unpin <repo> && git pin <repo>` 切换流程，成功输出保持可诊断。

## Migration Plan

1. 扩展共享 launcher 数据模型与 fake backend，保证旧 fixture 默认表示固化的 `code`。
2. 实现 Git config 读取及通用 executable 解析，先在应用层测试默认值、作用域优先级和错误隔离。
3. 逐平台替换 Code 专用创建逻辑，并验证旧 launcher 的 inspect/list/unpin/prune 兼容性。
4. 修正帮助文本、真实 Git 分派测试及中英文 README，发布为 v1.2。
5. 回滚可恢复 v1.1 binary；v1.2 创建的非 Code launcher 仍是合法平台入口，但旧 binary 未必能完整 inspect，必要时用户可用平台界面删除或在回滚前 unpin。
