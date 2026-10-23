# Tasks

## 1. 版本常量与解析层

- [x] 1.1 在 [`src/lib.rs`](src/lib.rs:1) 或 [`src/app.rs`](src/app.rs:1) 中新增公开版本常量，取值为 `env!("CARGO_PKG_VERSION")`，并验证 `cargo build` 通过且常量值与 [`Cargo.toml`](Cargo.toml:3) 的 `version` 一致
- [x] 1.2 在 [`src/cli.rs`](src/cli.rs:26) 的 `Invocation` 中新增 `Version` 变体，并在 `Operation::Pin` 分支把 `--version` 与 `-v` 映射到该变体，验证 `cargo test --lib cli` 中新增的解析用例通过
- [x] 1.3 更新 [`src/cli.rs`](src/cli.rs:16) 的 `Operation::Pin::usage()` 与 [`PIN_HELP`](src/cli.rs:35)，加入 `--version`/`-v` 说明，验证 `git pin -h` 输出包含版本入口且退出码为 0
- [x] 1.4 扩展 [`src/cli.rs`](src/cli.rs:71) 的单元测试矩阵：`--version`、`-v` 归入有效调用，`--version --list`、`-v path`、`--version -h` 归入用法错误，验证 `cargo test --lib cli` 全部通过

## 2. 执行层与无副作用短路

- [x] 2.1 在 [`src/app.rs`](src/app.rs:276) 的 `run()` 中于 `Invocation::Help` 之后、`Platform::current()` 之前处理 `Invocation::Version`，打印 `git-pin <version>` 并返回 `Ok(())`，验证 `cargo build` 通过且 `run()` 的 `match` 保持穷尽
- [x] 2.2 在非 Git repository 的临时目录中运行 `git-pin --version`，验证 stdout 为单行 `git-pin <version>`、stderr 为空、退出码为 0，且未创建任何启动器文件

## 3. 跨进程测试

- [x] 3.1 在 [`tests/process.rs`](tests/process.rs:39) 中新增测试，断言 `git-pin --version` 与 `git-pin -v` 的 stdout 等于 `format!("git-pin {}\n", env!("CARGO_PKG_VERSION"))`、stderr 为空、退出码为 0，验证 `cargo test --test process` 通过
- [x] 3.2 在 [`tests/process.rs`](tests/process.rs:39) 中新增测试，经真实 `git pin --version` 与 `git pin -v` 分派验证输出与退出码，确认 Git 前端不会拦截版本选项
- [x] 3.3 在 [`tests/process.rs`](tests/process.rs:39) 中新增测试，断言 `git pin --version --list`、`git pin -v path/to/repo`、`git pin --version -h` 以非零状态退出、stderr 含用法信息、stdout 不含版本号，验证 `cargo test --test process` 通过

## 4. 文档

- [x] 4.1 更新 [`README.md`](README.md) 的用法段落，加入 `git pin --version` / `git pin -v` 说明，验证文档中的命令与实际输出一致
- [x] 4.2 更新 [`README_zh.md`](README_zh.md) 的用法段落，与英文版保持一致，验证文档中的命令与实际输出一致

## 5. 集成验证

- [x] 5.1 运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 与 `cargo test`，验证全部通过
- [x] 5.2 运行 `openspec validate add-version-flag --strict`，验证变更通过校验
