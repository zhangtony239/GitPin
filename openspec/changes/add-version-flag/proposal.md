## Why

用户与脚本目前无法从命令行确认已安装的 Git Pin 版本，只能通过包管理器或可执行文件元数据间接推断。缺少版本查询入口使问题报告、兼容性排查和安装校验都变得困难，而 `git pin` 已经具备 `-h`、`--list`、`--prune` 这类无副作用查询模式，版本查询是同一类能力的自然补充。

## What Changes

- `git pin` 新增 `--version` 与 `-v` 两个等价选项，向标准输出打印当前可执行文件版本号并以零状态退出。
- 版本号 MUST 来自构建时的包版本（`Cargo.toml` 的 `version`），不得硬编码在源码中，避免发布版本与输出不一致。
- 版本查询是无副作用模式：不读取 Git 配置、不解析 repository、不访问平台启动器目录、不创建或删除任何启动器。
- `--version`/`-v` 与位置参数、`--list`、`--prune`、`-h`/`--help` 互斥；组合使用按既有用法错误处理，输出用法信息并以非零状态退出。
- `git pin -h` 与 `git-pin --help` 的帮助文本新增版本查询入口说明。
- 同步更新中英文 README 的命令用法说明。
- 范围假设：本次只覆盖 `git pin`；`git unpin` 不新增版本选项，保持现有参数矩阵不变。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `repository-pinning`: 修改「Git 外部命令接口」需求，在 `git pin` 的可接受参数集合中加入 `--version`/`-v`，并规定其输出、退出状态、无副作用语义与互斥规则。

## Impact

- [`src/cli.rs`](src/cli.rs)：`Operation::usage()` 用法文本、`Invocation` 新增版本变体、`parse()` 选项分派与单元测试矩阵。
- [`src/app.rs`](src/app.rs)：`run()` 在访问平台后端与 Git 配置之前处理版本查询，保持与 `Help` 相同的短路位置。
- [`src/bin/git-pin.rs`](src/bin/git-pin.rs)：无需改动，沿用现有 `parse` + `app::run` 流程。
- [`tests/process.rs`](tests/process.rs)：新增跨进程测试，验证 `git pin --version`、`git pin -v` 的输出与退出状态，以及组合参数的拒绝行为。
- 文档：[`README.md`](README.md) 与 [`README_zh.md`](README_zh.md) 的用法段落。
- 不新增依赖、不新增配置文件、不改变 launcher 存储布局或平台后端接口。
