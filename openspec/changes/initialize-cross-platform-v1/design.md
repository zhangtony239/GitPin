## Context

当前仓库仅包含项目备忘录与 OpenSpec 目录，没有 Rust 项目或既有兼容负担。行为契约见 `specs/repository-pinning/spec.md` 与 `specs/portable-distribution/spec.md`。

关键约束是：两个短生命周期 Git 外部命令、三平台原生启动器、无额外 registry、无需本地 Rust toolchain、所有编译验证进入 CI。实现过程需要小步推进；每一步都应能独立推送并由 CI 给出可信反馈，而不是积累到最后一次性联调。

## Goals / Non-Goals

**Goals:**

- 让 core repository 语义只实现一次，并以窄接口连接三套平台 backend。
- 让路径、desktop-entry、bundle metadata 等易错转换可在非目标平台进行纯逻辑测试；原生 API 集成在对应 CI runner 验证。
- 让两个 binary 共享应用层入口、错误模型和退出码，避免行为漂移。
- 让打包、合规与发布成为可重复的 CI 过程，并支持失败后安全重跑。

**Non-Goals:**

- 不为未来参数扩展引入大型 CLI framework 或稳定 library API。
- 不设计配置文件、独立 metadata database、自动安装、PATH 修改或自动更新。
- 不模拟所有桌面环境，也不保证 launcher 立即出现在每个第三方 launcher 的缓存中。
- 不在本机复制 CI 编译环境；本地允许只做文档与静态审阅。

## Decisions

### 1. 单 crate、library core、两个薄 binary

采用一个 Rust package：共享逻辑位于 library modules，`src/bin/git-pin.rs` 与 `src/bin/git-unpin.rs` 只选择操作并把 process arguments/working directory 传给统一应用入口。建议模块边界为：

- `cli`: 极简参数计数、帮助与 operation 选择；
- `repo`: 调用 Git、规范 root、生成 repository name；
- `launcher`: `LauncherBackend` trait、受管 launcher 描述及冲突语义；
- `platform`: Windows/Linux/macOS 条件编译 backend；
- `error`: 可展示错误类别与稳定退出码；
- `app`: pin/unpin orchestration，不含平台细节。

选择单 crate 是因为 V1 规模小，workspace 或多个内部 crate 会增加发布和版本管理成本。两个完全独立 crate 会复制核心逻辑；单 binary 根据可执行文件名分派则让直接测试和发布入口更隐晦，均不采用。

### 2. 用 `git rev-parse` 作为 repository 真相来源

通过 `git -C <input> rev-parse --show-toplevel` 获取 root；无参数时以当前目录作为 input。使用结构化 process invocation，不拼 shell 命令。输出按平台路径规则转为绝对路径，并保留 Git 返回路径指向的工作树身份。

不直接向上扫描 `.git`：这会错误处理 worktree、gitfile、submodule 与未来 Git 布局。V1 不引入 libgit2，因为会增加原生依赖、编译时间和许可证审查面，且用户调用 `git pin` 已天然依赖 Git。

### 3. 平台 backend 以窄 trait 隔离，受管文件携带可读回目标的信息

Core 向 backend 传递 `Repository { root, name }`，backend 提供 `inspect(name)`、`create(repo)`、`remove(expected)` 和 launcher location。冲突决策主要由 core 完成，backend 负责可靠读取平台格式中的目标 root。

V1 不建 JSON registry；`.lnk`、`.desktop` 和 `.app` 自身即 registry。每种格式都必须能读回由本工具写入的 repository root，以区分重复 pin 和同名冲突。只识别工具专属目录/文件前缀，并在删除前验证格式和目标，避免伤及用户自建入口。

替代方案是 sidecar JSON，它能简化查询，但引入双写一致性、损坏恢复和迁移问题，与 V1 简洁原则相悖。

### 4. 创建采用“同目录临时产物 + 原子替换/重命名”

Backend 先在最终父目录写入唯一临时文件或 bundle，完整校验后提交为最终名称。冲突检查后若最终目标发生竞态变化，提交必须失败而不是覆盖。Windows COM 保存到临时 `.lnk`；Linux 写临时 `.desktop` 后设置权限并 rename；macOS 构建临时 `.app` 目录后 rename。

删除只作用于已验证的受管产物。这样不需要锁文件即可避免大部分半成品；跨进程同名 pin 仍以最终原子提交的冲突结果为准。全局锁虽能简化竞态，但会带来 stale-lock 恢复，V1 不采用。

### 5. Windows 使用 `windows` crate 调用 Shell Link COM

Windows backend 初始化 COM，使用 `IShellLinkW` 设置 Code executable、单一 repository 参数、工作目录和 icon，再由 `IPersistFile` 保存。Inspect 同样通过 Shell Link API 读取参数与工作目录，不手工解析二进制 `.lnk`。路径使用宽字符串，并按 Windows 语义比较规范路径。

VS Code 解析按确定顺序尝试可验证候选：`code`/`code.cmd` 对应安装、用户级与系统级标准安装路径；最终必须获得现存的 Code executable，且写入快捷方式的是实际 GUI executable，不依赖启动时 shell 解析。具体候选可扩展，但失败须可诊断。

不使用 PowerShell 或 `WScript.Shell`，避免额外 runtime、quoting 和策略限制。

### 6. Linux 写 XDG Desktop Entry，不经 shell

目录为 `${XDG_DATA_HOME}/applications`，未设置时使用 `$HOME/.local/share/applications`。文件名采用工具前缀和经过文件名安全校验的 repository name。`Exec` 值依据 Desktop Entry Specification 对每个参数编码，直接调用解析出的 `code` executable 并传 root；不写 `sh -c`。文件中增加工具自有 key（例如原始 root 和格式版本）用于可靠 inspect，同时正确转义值。

写入后设置用户可执行权限；若 `update-desktop-database` 存在则 best-effort 调用。它缺失或刷新失败仅警告，不回滚一个本身有效的 entry。

替代方案是调用 `code <path>` 的 shell 脚本，文件更简单但扩大注入面并增加额外受管文件，因此不采用。

### 7. macOS 使用最小原生 `.app` bundle 与 `open -a`

每个 repository 生成 `~/Applications/Git Pin/<name>.app`，包含 `Contents/Info.plist`、`Contents/MacOS/git-pin-launcher` 与资源。Launcher 是随发布构建的当前架构辅助 binary（可由主 crate 增加内部 binary target），repository root 存入 plist 自有 key；辅助 binary 从所属 bundle 的 plist 读取 root，以参数数组调用 `/usr/bin/open -a "Visual Studio Code" --args <root>`。不通过 shell，不把路径嵌入脚本文本。

Bundle identifier 使用反向域名前缀加 repository name 的稳定哈希，避免不同名称编码造成冲突；显示名称仍保留 basename。Info.plist 使用标准序列化库或受控 XML writer 生成，不做字符串模板替换。创建后可 best-effort 触发 Launch Services 注册，但 `.app` 有效性不依赖刷新成功。

采用 `.app` 而非 alias/Automator，前者是 Finder、Spotlight 和 Launch Services 的标准用户应用单元，可完全由项目生成。直接复制 shell script 到 Applications 不能提供等价应用体验。

### 8. 依赖保持最小并在 target scope 声明

优先使用标准库。通用依赖只选择成熟的小型 crate 处理错误上下文、临时文件/目录和必要序列化；Windows API dependency 放在 Windows target section。参数规模不足以证明引入大型 CLI parser 的收益。

在 lockfile 提交后，先让功能 CI 可运行，最后在 release job 加入许可证与 advisory 扫描。公开 V1 Release 以 MIT 为项目许可证，并将“扫描通过”设为打包发布前置条件。工具版本在 workflow 中固定，扫描策略配置提交仓库，减少供应链漂移。

### 9. 测试分层，并把真实平台集成放到 CI

- 单元测试：参数矩阵、name 校验、路径比较策略、Desktop Entry escaping、plist 内容、包名/version 规则；
- 使用临时目录和 fake backend 的应用测试：幂等、冲突、unpin 路径/名称分派、失败不留半成品；
- 使用临时 Git repository 的 process integration test：root discovery、子目录和 worktree；
- 平台 runner integration test：在隔离的临时 launcher root（backend 测试注入）创建、inspect、删除原生格式，避免污染 runner 用户真实菜单；
- packaging test：解压 ZIP，验证目录、文件名、权限、版本和 checksum。

平台路径允许由测试专用依赖注入覆盖，但 production constructor 始终使用规范系统目录。这样既验证真实原生格式，又避免在 CI 用户环境留下入口。

### 10. CI 分为快速门禁、平台验证、release 三层

每次 PR 先执行格式与通用测试，再以原生 OS matrix 执行 clippy、完整测试和 release build。任务实现按依赖顺序小步提交，每个提交都推送 CI；后续步骤只基于绿色提交扩展。

Release 由 `v*` tag 触发，校验 tag 与 Cargo version，一次构建两个主 binary（macOS 另带内部 launcher）、按 OS/arch staging、复制 README/LICENSE、压缩、复验、计算 SHA-256。matrix 全部成功并通过 dependency license/advisory gate 后，单独 publish job 下载所有 artifact 并创建 Release，避免部分发布。

架构支持以 GitHub-hosted runner 和 Rust target 的实际可用性为准：首轮实现先建立三个原生 OS 的 x86_64/runner-native 产物，再逐项启用可在对应 OS 原生 runner 可靠构建和测试的 arm64。未通过验证的组合不进入公开矩阵。

## Risks / Trade-offs

- [不同 Linux desktop environment 对刷新、可执行位和图标处理不一致] → 遵循 Desktop Entry/XDG 标准，以文件有效性为验收核心，将数据库刷新降为 best-effort，并在多个可用 runner/手工环境补充验证。
- [macOS 未签名 bundle 可能出现安全提示，且 Spotlight 索引存在延迟] → bundle 仅包装本地用户主动安装的同一发布 binary，不下载执行内容；记录限制，V1 验收 Finder 可启动和 bundle 可索引，不承诺即时出现。后续可独立引入签名/notarization。
- [VS Code 安装发现因渠道版、Insiders 或用户自定义位置而失败] → V1 只支持稳定版可验证候选并给出明确错误；不静默写无效入口。渠道选择以后通过显式 CLI/config 提案扩展。
- [没有独立 registry 使外部手工编辑 launcher 后 inspect 失败] → 将格式版本和 root 写入平台产物，严格验证；损坏时报告修复建议而不猜测或覆盖。
- [unpin 单个参数既可能是路径也可能是名称] → 明确定义“现存路径优先，否则精确名称”；仓库已删除时仍可按 basename 清理。
- [无本地编译链会拉长反馈周期] → 小步提交、先快后全的 CI job、依赖缓存和并行 OS matrix；不以跳过检查换取速度。
- [三平台首个变更范围较大] → 先完成可 fake 的 core contract，再按 Linux、Windows、macOS 独立 backend 递增；每个 backend 具有独立 CI 里程碑，最后再合并 release gate。
- [上游依赖许可证或 advisory 在最后扫描时阻塞 release] → 依赖从一开始保持最小且优先宽松许可证；最终门禁前先生成依赖清单，发现问题替换 dependency，不降低策略。

## Migration Plan

1. 初始化 MIT 许可的 Rust package、两个空薄 binary 和基础 CI；此时不发布版本。
2. 以 fake backend 完成 core/repository contract 和通用测试，持续保持 CI 绿色。
3. 依次加入 Linux、Windows、macOS backend 与各自 runner integration tests；未完成平台不标记支持。
4. 加入 README 和 portable packaging，CI 生成非公开 workflow artifacts 供检查。
5. 扫描全部上游依赖许可证与 advisories，修复或替换不满足策略的 dependency。
6. 启用 tag release；只有三平台必需矩阵和所有门禁均通过才公开 V1 资产。

在首次公开版本前无需数据迁移。若任一阶段失败，回滚到最后一个绿色小步提交；release publish job 之前的失败不会产生部分公开版本。已经生成的测试 launcher 由测试 teardown 删除，必要时可按工具专属目录/前缀安全清理。
