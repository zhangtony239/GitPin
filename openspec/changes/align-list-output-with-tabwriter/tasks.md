# Tasks

## 1. 依赖与供应链登记

- [x] 1.1 在 [`Cargo.toml`](Cargo.toml:21) 的 `[dependencies]` 中加入 `tabwriter = "1.4.1"`，运行 `cargo build` 确认构建成功且 `Cargo.lock` 新增 `tabwriter` 与 `unicode-width` 两项
- [x] 1.2 更新 [`DEPENDENCIES.md`](DEPENDENCIES.md:11)：表格新增 `tabwriter`（direct）与 `unicode-width`（transitive）两行，并把「唯一的直接依赖是 `windows`」的表述改为同时包含 `tabwriter`，确认清单与 `Cargo.lock` 条目一一对应
- [x] 1.3 人工核对 [`deny.toml`](deny.toml:1) 策略（本机未安装 cargo-deny，实际 `cargo deny check` 由 CI 执行）：`tabwriter` 的旧式 `Unlicense/MIT` 按 `Unlicense OR MIT` 解析，`MIT` 已在 `allow` 内；`unicode-width` 为 `MIT OR Apache-2.0`，同样在 `allow` 内；`Cargo.lock` 中 `unicode-width` 仅 0.2.2 一个版本，不触发 `multiple-versions`；`tabwriter = "1.4.1"` 非通配符，来源为 crates.io 允许的 registry。因此无需修改 `deny.toml`

## 2. 列表渲染实现

- [x] 2.1 在 [`src/app.rs`](src/app.rs:324) 中新增私有渲染函数 `write_list<W: Write>(writer: W, records: &[LauncherRecord]) -> Result<(), AppError>`，用 `TabWriter` 按 `name\troot\tstatus` 写入并在返回前 `flush()`，确认 `cargo build` 通过
- [x] 2.2 将 `Invocation::List` 分支改为调用该渲染函数并传入 `io::stdout()`，保留 `records.is_empty()` 时输出 `no pinned repositories` 的短路分支，确认 `cargo build` 通过
- [x] 2.3 保持 `batch_error("list", report.errors)` 的调用位置与语义不变，确认 `--list` 在存在枚举/自检错误时仍先输出成功项、再向标准错误报告、最后以非零状态退出

## 3. 渲染单元测试

- [x] 3.1 在 [`src/app.rs`](src/app.rs:366) 的测试模块中新增单元测试：构造长度差异明显的多个 `LauncherRecord`（例如 3 字符与 20 字符的 name），断言渲染结果中每行 root 列的起始字节位置一致，确认 `cargo test` 通过
- [x] 3.2 新增单元测试：构造含非 ASCII 字符（如中文）的 name 与 root，断言后续列按显示宽度对齐而非字节数，确认 `cargo test` 通过
- [x] 3.3 新增单元测试：混合有效项与 `RootStatus::Invalid(reason)` 项，断言两类行的 name 列与 root 列对齐一致且 `invalid: <reason>` 完整出现在状态列，确认 `cargo test` 通过

## 4. 进程级测试与文档

- [x] 4.1 扩展 [`tests/process.rs`](tests/process.rs:354) 的 `list_is_read_only_and_prune_is_diagnostic_and_idempotent`：pin 两个名称长度不同的 repository 后运行 `--list`，断言两行中 root 列起始位置一致，且仍包含 `valid`，确认 `cargo test --test process` 通过
- [x] 4.2 核对 [`README.md`](README.md:1) 与 [`README_zh.md`](README_zh.md:1)：两份文档只列出 `git pin --list` 命令本身，没有示例输出，因此无需改动；按用户要求不为该细节新增示例输出

## 5. 集成验证

- [x] 5.1 运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 与 `cargo test`，确认全部通过
- [x] 5.2 在真实环境运行 `git pin --list`：9 个受管启动器全部按 name 排序且三列对齐（name 列宽由 `ThroneForAndroid` 决定），退出码 0。真实环境当前没有无效项，无效行与有效行混排的对齐由 [`tests/process.rs`](tests/process.rs:354) 的进程级测试覆盖（pin 两个不同长度名称后删除其中一个再 `--list`）
