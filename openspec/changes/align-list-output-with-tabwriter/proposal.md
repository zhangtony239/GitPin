# Proposal

## Why

`git pin --list` 目前用单个 `\t` 分隔 name、root 和状态三列。终端与管道消费方对制表位的处理不一致：制表位宽度固定为 8 列，当 repository name 长度超过一个制表位宽度（或包含非 ASCII 字符）时，后续列会错位，输出在视觉上无法对齐，也难以用固定列宽解析。用户需要一份列边界稳定、可读的列表输出。

## What Changes

- 将 `git pin --list` 的表格渲染从手写 `\t` 拼接改为基于 `tabwriter` 的列对齐渲染，列宽按本次输出的实际内容计算。
- 保持现有列语义与顺序不变：repository name、记录的 repository root、有效性状态（`valid` 或 `invalid: <reason>`）。
- 保持现有行为不变：空列表提示、按 name 的确定性排序、只读语义、部分失败时继续输出其余项并以非零状态退出。
- 新增一个直接依赖 `tabwriter`（及其传递依赖 `unicode-width`），并同步更新依赖清单与供应链策略。
- 非 BREAKING：列内容、退出码和标准错误行为均不变；仅列间空白由制表符变为按内容对齐的空格填充。

## Capabilities

### New Capabilities

无。本变更不引入新的系统行为，只调整既有列表输出的呈现方式。

### Modified Capabilities

- `repository-pinning`: 「受管启动器列表与有效性自检」要求新增列对齐输出约束——每项 SHALL 以对齐的列呈现 name、root 与状态，列宽按本次输出内容计算，且对齐 SHALL 在 name 或 root 含非 ASCII 字符时仍成立。

## Impact

- 代码：`src/app.rs` 中 `Invocation::List` 的输出分支（改为通过 `tabwriter::TabWriter` 写入标准输出）。
- 依赖：`Cargo.toml` 新增 `tabwriter`；`Cargo.lock` 新增 `tabwriter` 与 `unicode-width`。
- 供应链：`DEPENDENCIES.md` 依赖清单与 `deny.toml` 许可证白名单需覆盖新增依赖（`tabwriter` 与 `unicode-width` 均为 `MIT OR Apache-2.0`）。
- 测试：`tests/process.rs` 中 `--list` 的进程级断言需覆盖对齐输出；`src/app.rs` 单元测试可覆盖渲染函数。
- 文档：`README.md` / `README_zh.md` 中若展示 `--list` 示例输出，需更新为对齐后的形式。
