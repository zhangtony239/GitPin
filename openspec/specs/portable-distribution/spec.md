# portable-distribution Specification

## Purpose

定义 `git-pin` 在无本地 Rust 编译链的开发模式下如何由持续集成验证，并为 Windows、Linux 与 macOS 生成可追溯、可直接加入 `PATH` 使用的便携发布包。

## Requirements

### Requirement: CI 是 Rust 变更的权威验证环境
每个影响 Rust 源码、构建配置、测试或打包流程的 pull request 和主分支提交 SHALL 由 GitHub Actions 执行格式检查、静态分析、测试和 release 构建。仓库 SHALL 不要求贡献者在本地安装 Rust toolchain；合并判定 SHALL 以受保护 CI 检查结果为准。

#### Scenario: Pull request 验证
- **WHEN** pull request 修改 Rust 源码或其构建、测试、打包配置
- **THEN** CI 执行等价于 `cargo fmt --check`、拒绝 warning 的 `cargo clippy`、`cargo test` 和 `cargo build --release` 的检查，任一失败均使验证失败

#### Scenario: 无本地 Rust 工具链贡献
- **WHEN** 贡献者只编辑代码并推送分支而本机未安装 Rust
- **THEN** 所有编译型验证均可在 GitHub Actions 完成，且仓库文档明确将 CI 结果作为合并前验证依据

### Requirement: 原生平台矩阵构建
系统 SHALL 由 `.github/workflows/release.yml` 在 Windows runner 构建 Windows 产物、Linux runner 构建 Linux 产物、macOS runner 构建 macOS 产物，并在同一 workflow 中完成发布暂存、ZIP 生成、包复验和 SHA-256 摘要生成。每个宣称支持的 operating-system/architecture 组合 MUST 由对应操作系统的原生 runner 构建或在该原生平台上使用受支持 target；不得把单一 Linux runner 的跨平台交叉编译作为 V1 发布依据。普通 CI MAY 验证源码、测试及发布所依赖的平台行为，但 SHALL NOT 作为公开发布 binary 或 ZIP 的来源。

#### Scenario: 三平台原生构建
- **WHEN** release workflow 为 V1 版本运行
- **THEN** Windows、Linux、macOS job 分别在各自原生 runner 上构建其平台产物

#### Scenario: 架构不可用
- **WHEN** 某宣称支持的架构缺少可用的原生或平台内受支持构建路径
- **THEN** release 明确失败或不宣称该架构受支持，而不是发布未经构建验证的占位包

### Requirement: Portable ZIP 包内容
每个平台和架构 SHALL 生成单独 ZIP，顶层目录名称 SHALL 与 ZIP 基名一致。Windows 包 SHALL 包含 `git-pin.exe`、`git-unpin.exe`、`README.md` 和 `LICENSE`；Linux 与 macOS 包 SHALL 包含 `git-pin`、`git-unpin`、`README.md` 和 `LICENSE`。Unix-like binary 解压后 SHALL 保留可执行权限。包不得包含安装器，且使用时不得要求管理员权限、修改 registry 或自动修改环境变量。

#### Scenario: Windows portable package
- **WHEN** 构建 Windows x86_64 release 包
- **THEN** ZIP 中存在同名顶层目录及两个 `.exe`、README 和 MIT LICENSE，用户解压并把目录加入 `PATH` 后可运行两个命令

#### Scenario: Unix-like portable package
- **WHEN** 构建 Linux 或 macOS release 包
- **THEN** ZIP 中存在同名顶层目录及两个保留可执行权限的 binary、README 和 MIT LICENSE，用户解压并把目录加入 `PATH` 后可运行两个命令

#### Scenario: macOS portable package 自包含创建 bundle
- **WHEN** 用户只解压 macOS ZIP 中规定的两个正式 binary、README 和 LICENSE，并从该目录运行 `git pin`
- **THEN** `git-pin` 可创建带有效可执行启动入口的 `.app` bundle，不依赖 ZIP 外部或未包含在发布包中的辅助 executable

### Requirement: 发布包命名与版本一致性
发布包 SHALL 使用 `git-pin-v<semver>-<platform>-<architecture>.zip` 命名，其中 platform 为 `windows`、`linux` 或 `macos`，architecture 使用仓库声明的稳定标识。`Cargo.toml` 的 `[package].version` SHALL 是唯一项目版本源；Git tag MUST 与该版本加 `v` 前缀后的值一致，包名和顶层目录名 SHALL 由该版本派生。README 及其他包内文档 SHALL NOT 作为机器可读版本源，也 SHALL NOT 参与构建版本校验。tag、Cargo package version 或派生包名不一致时 release SHALL 在发布前失败。

#### Scenario: 正常版本发布
- **WHEN** tag `v1.2.3` 触发 Windows x86_64 打包
- **THEN** 产物名为 `git-pin-v1.2.3-windows-x86_64.zip`，其顶层目录名为 `git-pin-v1.2.3-windows-x86_64`

#### Scenario: 版本不一致
- **WHEN** release tag 与项目元数据版本不一致
- **THEN** workflow 在创建公开 Release 或上传最终资产前失败并报告不一致值

#### Scenario: README 不声明版本
- **WHEN** README 不包含当前版本文本，或其说明性文本未随 package version 变化
- **THEN** release 仍仅根据 `Cargo.toml` 的 `[package].version` 校验 tag 并派生包名，不解析 README 获取版本

### Requirement: MIT 许可与上游合规扫描
项目 V1 SHALL 最终以 MIT License 分发。在最终 release 打包前，CI MUST 扫描直接和传递 Rust dependencies 的许可证与已知安全公告，并 SHALL 阻止许可证与 MIT 分发目标不兼容、许可证元数据未知且未被明确审核，或命中按仓库策略禁止的安全公告的依赖进入公开包。早期实现提交可以先建立功能，但公开 V1 Release MUST 等待该合规门禁通过。

#### Scenario: 依赖合规通过
- **WHEN** 所有上游依赖许可证兼容且安全公告扫描未发现被禁止问题
- **THEN** release workflow 允许继续打包包含 MIT `LICENSE` 的资产

#### Scenario: 上游许可证不兼容或未知
- **WHEN** 扫描发现不兼容许可证，或无法识别且未经显式审核的依赖许可证
- **THEN** release 在公开资产前失败，并报告 dependency 及其许可证状态

#### Scenario: 上游安全公告命中
- **WHEN** dependency advisory 扫描发现仓库策略禁止的已知漏洞或不再维护风险
- **THEN** release 在公开资产前失败，并报告受影响 dependency 与 advisory

### Requirement: 发布资产完整性
Release workflow SHALL 为每个 ZIP 生成可公开校验的 SHA-256 摘要，并仅在所有必需平台 job、测试、合规扫描、包内容校验和摘要生成成功后发布同一版本资产。

#### Scenario: 完整 release
- **WHEN** 所有平台构建和门禁通过
- **THEN** GitHub Release 同时包含各支持矩阵 ZIP 及其 SHA-256 校验信息

#### Scenario: 部分平台失败
- **WHEN** 任一必需平台构建或包校验失败
- **THEN** workflow 不发布看似完整的 V1 Release，并明确标记失败矩阵项
