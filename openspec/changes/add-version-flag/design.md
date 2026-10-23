# Design

## Context

见 [`proposal.md`](openspec/changes/add-version-flag/proposal.md) 的 Why 与 What Changes。

当前实现的相关约束：

- [`src/cli.rs`](src/cli.rs:38) 的 `parse()` 只接受零个或一个参数，先检查参数个数，再按 `Operation` 分派到固定选项字面量；`Invocation` 是封闭枚举，`app::run()` 对其做穷尽匹配。
- [`src/app.rs`](src/app.rs:276) 的 `run()` 在 `Invocation::Help` 时于任何环境访问之前短路返回，这是唯一不触碰平台后端与 Git 配置的路径。
- [`src/bin/git-pin.rs`](src/bin/git-pin.rs:13) 只做 `parse` + `app::run`，不含业务逻辑，因此版本查询不需要改动入口文件。
- 包版本唯一来源是 [`Cargo.toml`](Cargo.toml:3) 的 `version` 字段，当前为 `1.2.0`。
- 现有跨进程测试 [`tests/process.rs`](tests/process.rs:39) 通过 `CARGO_BIN_EXE_git-pin` 启动真实二进制并断言 stdout/stderr 与退出状态。

## Goals / Non-Goals

**Goals:**

- 版本号只有一个事实来源，随包版本自动变化。
- 版本查询与 `-h` 一样是纯查询路径，不依赖 Git、repository 或平台启动器目录。
- 参数解析保持"单参数、互斥模式"的既有模型，不引入子命令或参数组合语法。

**Non-Goals:**

- 不实现 `--verbose`、`--quiet` 等日志级别选项，也不为 `-v` 预留其他语义。
- 不为 `git unpin` 增加版本选项。
- 不输出构建元数据（commit hash、构建时间、目标三元组），也不引入 `build.rs` 或版本注入依赖。
- 不改变 `--list`、`--prune`、`-h` 的现有行为与输出格式。

## Decisions

### 版本来源：编译期 `CARGO_PKG_VERSION`

在库内暴露一个常量，取值为 `env!("CARGO_PKG_VERSION")`，由 `app::run()` 打印。Cargo 在编译时把 `Cargo.toml` 的 `version` 注入该环境变量，因此发布版本与输出天然一致，无需手工同步。

替代方案：

- 运行时读取 `Cargo.toml`：发布产物中不存在该文件，且会引入文件系统依赖与失败路径。
- 手写版本常量：需要人工同步，容易与包版本漂移。
- `build.rs` 注入 git 描述：超出本次范围，且会破坏可复现构建与无依赖发布边界。

### 输出格式：`git-pin <version>`

单行输出 `git-pin 1.2.0` 到标准输出，末尾换行，退出码 0。前缀使用可执行文件名而非 `git pin`，使输出在直接调用 `git-pin --version` 与经 Git 分派两种情况下都准确，并符合常见 CLI 约定（`git --version`、`cargo --version` 均为 `<name> <version>`）。

替代方案：只输出裸版本号 `1.2.0`。虽然更易被脚本 `cut` 解析，但缺少程序标识，且与生态惯例不一致；脚本仍可用 `awk '{print $2}'` 提取。

### 解析层：新增 `Invocation::Version` 变体

在 `Operation::Pin` 的分支中，把 `--version` 与 `-v` 映射到新的 `Invocation::Version`，与 `--list`、`--prune`、`-h` 处于同一层。由于 `parse()` 已先拒绝多于一个参数，`--version --list`、`-v path` 等组合会自动落入既有用法错误路径，无需新增互斥检查代码。

替代方案：在 `parse()` 之前于 `main` 中扫描 `--version`。这会绕过统一的参数校验，使 `git pin --version --list` 静默成功，违反 spec 中的互斥场景。

### 执行层：与 `Help` 并列短路

`app::run()` 在 `Invocation::Help` 之后、`Platform::current()` 与 `NativeBackend::new()` 之前处理 `Invocation::Version`。这样版本查询在非 repository 目录、无 Git、启动器目录不可写时仍成功，满足"无副作用"场景。

替代方案：在 `run()` 的 `match` 内处理。此时平台后端已构造，会在部分平台上产生目录探测或失败，破坏无副作用契约。

### 帮助与用法文本

`Operation::Pin::usage()` 与 `PIN_HELP` 增加 `--version`/`-v` 行，说明其打印版本并退出。`PIN_HELP` 的 MODES 段落新增一行，保持与现有 `-h` 条目相同的措辞风格。

### 测试策略

- `src/cli.rs` 单元测试：把 `--version`、`-v` 加入有效调用矩阵；把 `--version --list`、`-v path`、`--version -h` 加入拒绝矩阵。
- `tests/process.rs` 跨进程测试：断言 `git pin --version` 与 `git pin -v` 的 stdout 等于 `format!("git-pin {}\n", env!("CARGO_PKG_VERSION"))`、stderr 为空、退出码 0；并在非 repository 临时目录中运行以证明无副作用。

## Risks / Trade-offs

- [`-v` 语义被占用](src/cli.rs:52) → 未来若需要 `--verbose`，`-v` 无法复用；在帮助文本中明确 `-v` 是 `--version` 的别名，避免歧义。
- 输出格式成为脚本契约 → 一旦发布，改变 `git-pin <version>` 前缀即为破坏性变更；在 spec 中固定该格式，后续变更需走 spec 修改流程。
- 版本号只反映包版本，不反映构建来源 → 用户报告问题时可能无法区分同版本的不同构建；本次明确列为 Non-Goal，若需要可另开变更引入构建元数据。
- 与 `git pin --help` 不同，`git pin --version` 不会被 Git 前端拦截 → 无需像帮助那样提供替代入口，但需在跨进程测试中经真实 `git pin` 分派验证，防止未来 Git 行为变化。
