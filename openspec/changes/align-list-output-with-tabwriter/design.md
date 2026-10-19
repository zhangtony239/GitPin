# Design

## Context

见 [`proposal.md`](openspec/changes/align-list-output-with-tabwriter/proposal.md) 的 Why 一节。当前实现位于 [`src/app.rs`](src/app.rs:324) 的 `Invocation::List` 分支：它直接对每条记录 `println!` 一行，用单个 `\t` 分隔 name、root 与状态。终端把 `\t` 展开到下一个 8 列制表位，因此列起点取决于前一个字段的显示宽度，长名称或非 ASCII 名称必然错位。

约束：

- 项目当前唯一的直接依赖是 target-scoped 的 `windows` crate，供应链策略 [`deny.toml`](deny.toml:1) 只允许 `Apache-2.0`、`MIT`、`Unicode-3.0`，且 `multiple-versions = "deny"`、`wildcards = "deny"`。
- [`DEPENDENCIES.md`](DEPENDENCIES.md:1) 是人工维护的依赖许可证清单，新增依赖必须同步登记。
- 列表输出是只读操作，且部分失败时仍需先输出成功项、再以非零状态退出（见 [`src/app.rs`](src/app.rs:345) 的 `batch_error`）。
- 进程级测试 [`tests/process.rs`](tests/process.rs:354) 通过捕获 stdout 断言 `--list` 行为。

## Goals / Non-Goals

**Goals:**

- 让 `--list` 的每一列在所有行中起始于相同的显示列位置，且列宽由本次输出的实际内容决定。
- 对齐在 name 或 root 含非 ASCII 字符时仍成立（按显示宽度而非字节数计算）。
- 保持列内容、列顺序、排序、空列表提示、退出码与标准错误行为完全不变。
- 让渲染逻辑可被单元测试直接覆盖，而不必启动子进程。

**Non-Goals:**

- 不引入表头行、颜色、边框或 `--json` 等新输出模式。
- 不改动 `--prune`、`pin`、`unpin` 的输出格式。
- 不为 `--list` 增加列宽上限、截断或换行策略。
- 不重构 `scan`/`list` 的数据模型。

## Decisions

### 决策 1：使用 `tabwriter` crate 而非手写列宽计算

采用 `tabwriter` 1.4.1（`TabWriter`，elastic tabstops）。它按 `\t` 切分单元格，缓冲整块输出后按每列最大显示宽度补齐空格，并借助 `unicode-width` 计算显示宽度，天然满足非 ASCII 对齐要求。

替代方案：

- **手写 `max_width` + `format!("{:<width$}")`**：`{:<width$}` 按 `char` 数量补齐，对全角字符（如中文）会算错宽度，需要自行引入 `unicode-width` 并手写补齐逻辑，代码量与出错面都更大。
- **固定列宽常量**：无法适应任意长度的 repository name 与 root，仍会错位。
- **保留 `\t` 但改用 `\t\t` 或空格填充**：只是把错位位置推后，不解决根本问题。

### 决策 2：把渲染抽成独立函数，接收 `impl Write`

在 [`src/app.rs`](src/app.rs:324) 中新增一个私有函数（例如 `write_list<W: Write>(writer: W, records: &[LauncherRecord]) -> io::Result<()>`），由 `Invocation::List` 分支传入 `io::stdout()` 调用。这样单元测试可以传入 `Vec<u8>` 断言精确输出，进程级测试继续覆盖端到端行为。

`TabWriter` 必须在函数返回前 `flush()`，否则缓冲内容不会写出；`flush` 的 I/O 错误需要映射为 `AppError`，与现有错误处理风格一致（`AppError::failure`）。

### 决策 3：列间使用 `\t` 作为 tabwriter 的分隔符，最小宽度与 padding 取默认值

`TabWriter::new(writer)` 的默认 `minwidth = 0`、`padding = 1`，即列间恰好一个空格。这既保证对齐，又不会让输出比现在明显变宽。不启用 `ansi_formatting` feature，避免引入 `regex`/`lazy_static` 等额外依赖。

### 决策 4：空列表与错误路径保持原样

`records.is_empty()` 时仍直接输出 `no pinned repositories`，不经过 `TabWriter`（避免无意义的缓冲与 flush）。`batch_error("list", report.errors)` 的调用位置与语义不变，保证「先输出成功项、再报错、最后非零退出」的顺序。

### 决策 5：依赖与供应链同步更新

- [`Cargo.toml`](Cargo.toml:21) 的 `[dependencies]` 新增 `tabwriter = "1.4.1"`（精确到次版本，不使用通配符）。
- `Cargo.lock` 新增 `tabwriter` 与 `unicode-width`（`unicode-width` 为 `MIT OR Apache-2.0`，已在白名单内）。
- `tabwriter` 的 crates.io 许可证字段是旧式 `Unlicense/MIT`。cargo-deny 将其解析为 `Unlicense OR MIT`，因 `MIT` 已允许而通过；若 CI 实际报许可证错误，则在 [`deny.toml`](deny.toml:13) 的 `allow` 中补 `Unlicense` 并在此记录理由，而不是放宽 `confidence-threshold`。
- [`DEPENDENCIES.md`](DEPENDENCIES.md:11) 表格新增两行，并把「唯一的直接依赖是 `windows`」的表述改为「直接依赖为 `windows`（Windows target only）与 `tabwriter`」。

## Risks / Trade-offs

- **输出缓冲导致部分输出延迟** → `TabWriter` 会缓冲到 flush 才写出。缓解：渲染函数在返回前显式 `flush()`，且 `--list` 的输出量级（几十行）不构成内存或延迟问题。
- **列宽随最长 root 增长，宽终端下仍可能折行** → 这是 elastic tabstops 的固有行为，也是「按内容对齐」的直接后果。缓解：不引入截断（截断会破坏可复制性），并在文档示例中体现真实形态。
- **新增依赖扩大供应链面** → 缓解：`tabwriter` 为 BurntSushi 维护的成熟 crate（无 unsafe 依赖链，仅一个传递依赖 `unicode-width`），并同步更新 `DEPENDENCIES.md` 与 `deny.toml` 审查。
- **`multiple-versions = "deny"` 可能因 `unicode-width` 出现多版本而失败** → 当前依赖图中没有 `unicode-width`，首次引入不会产生多版本冲突；若未来冲突，需在 `deny.toml` 中显式记录 skip 理由。
- **进程级测试对空白的断言可能过严** → 现有 [`tests/process.rs`](tests/process.rs:354) 只断言 `contains("valid")` 等子串，不依赖具体空白；新增断言应针对「列起始位置一致」而非硬编码整行字符串，避免测试对 padding 细节过敏。

## Migration Plan

1. 添加依赖并更新 `Cargo.lock`、`DEPENDENCIES.md`、`deny.toml`（如需）。
2. 引入渲染函数并切换 `Invocation::List` 分支。
3. 补充单元测试与进程级测试，运行 `cargo test`、`cargo clippy`、`cargo deny check`。
4. 更新 `README.md` / `README_zh.md` 中 `--list` 的示例输出。

回滚策略：本变更不涉及持久化数据或平台启动器格式，回滚只需还原 `src/app.rs` 的输出分支与依赖声明，已存在的启动器不受影响。

## Open Questions

无。
