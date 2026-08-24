## 1. 项目骨架与首个 CI 反馈

- [x] 1.1 创建单 Rust package、提交 lockfile，并建立 library modules、`git-pin`/`git-unpin` 两个薄 binary 以及仅在 macOS 构建的内部 launcher target；此提交只要求三个原生 OS runner 均可编译
- [x] 1.2 添加 MIT `LICENSE`、最小 `README.md`、repository ignore/settings，并说明本地无需 Rust、所有编译验证以 GitHub Actions 为准
- [x] 1.3 添加 PR/main CI workflow，在 Windows、Linux、macOS 原生 runner 执行 fmt check、拒绝 warning 的 clippy、test 与 release build，并启用 Cargo 缓存；推送并确认首轮矩阵绿色

## 2. 核心命令与 Repository 发现

- [x] 2.1 实现零个或一个位置参数的命令解析、稳定 operation 类型、用法错误和退出码，并以单元测试覆盖两个 binary 的有效/无效参数矩阵
- [x] 2.2 以结构化 `git -C <input> rev-parse --show-toplevel` 实现 repository root 发现、绝对路径校验和上下文错误，覆盖当前目录、给定子目录、worktree、非仓库及 Git 不可用测试
- [x] 2.3 实现 basename 命名、三平台安全名称校验及平台路径等价策略，覆盖空格、非 ASCII、非法文件名与 Windows 大小写场景
- [x] 2.4 推送 core parsing/repository 小步并以三平台 CI 修复所有路径差异，保持 release build 绿色后再进入 launcher orchestration

## 3. Launcher 抽象与应用语义

- [x] 3.1 定义 `Repository`、受管 launcher inspection 结果和窄 `LauncherBackend` trait，并允许测试注入 launcher root 而 production constructor 固定系统目录
- [x] 3.2 实现共享 pin orchestration：VS Code 依赖确认、首次创建、同 root 幂等、不同 root 冲突、原子提交失败清理和可诊断错误
- [x] 3.3 实现共享 unpin 分派与语义：无参数当前仓库、现存路径优先、否则精确名称、目标缺失幂等、按路径目标不匹配时拒绝删除
- [x] 3.4 使用 fake backend 和临时目录覆盖创建/读取/删除失败、竞态冲突、不留半成品、不删除非受管产物及 stdout/stderr/exit-code 契约；推送并确认 CI 绿色

## 4. Linux XDG Backend

- [x] 4.1 实现 Linux VS Code executable 查找、`XDG_DATA_HOME`/HOME 目录解析、工具前缀文件名和受管 Desktop Entry metadata
- [x] 4.2 实现符合 Desktop Entry Specification 的字段与 `Exec` 参数编码、临时文件原子提交、用户可执行权限、inspect 与安全删除
- [x] 4.3 添加 `update-desktop-database` best-effort 刷新及 warning 行为，确保工具缺失或刷新失败不撤销有效入口
- [x] 4.4 在 Linux CI 的隔离 launcher root 上集成测试创建、读回、冲突、删除及空格/特殊字符/非 ASCII 路径，并推送确认矩阵绿色

## 5. Windows Shell Link Backend

- [x] 5.1 在 Windows target scope 添加最小 `windows` crate features，实现 COM 初始化/释放和稳定版 Visual Studio Code GUI executable 候选解析
- [x] 5.2 使用 `IShellLinkW` 与 `IPersistFile` 实现 `.lnk` 的 target、单参数 arguments、working directory、icon 和受管 root 读回
- [x] 5.3 实现 `%APPDATA%` 启动器目录、宽字符串路径、Windows 路径比较、临时 `.lnk` 原子提交与验证后删除
- [ ] 5.4 在 Windows CI 的隔离 launcher root 上集成测试真实 Shell Link 创建/inspect/unpin、同名冲突及空格/非 ASCII 路径，并推送确认矩阵绿色

## 6. macOS Application Bundle Backend

- [ ] 6.1 实现 `~/Applications/Git Pin` 目录、稳定哈希 bundle identifier、受控 `Info.plist` 序列化及受管 root/格式版本 metadata
- [ ] 6.2 实现内部 `git-pin-launcher` 从所属 bundle 读取 root，并以参数数组调用 `/usr/bin/open -a "Visual Studio Code" --args <root>`，禁止 shell 插值
- [ ] 6.3 实现临时 `.app` 目录组装、当前架构 launcher 安装与权限、bundle inspect、原子 rename、安全删除及 Launch Services best-effort 注册
- [ ] 6.4 在 macOS CI 的隔离 Applications root 上集成测试 bundle 结构、plist、launcher 调用参数、冲突、unpin 及特殊字符路径，并推送确认矩阵绿色

## 7. 端到端行为与文档

- [ ] 7.1 为两个正式 binary 添加 process-level 测试，验证 Git 外部命令分派、当前目录/路径/name 输入、幂等、冲突、错误上下文和退出状态
- [ ] 7.2 在各原生 CI runner 运行隔离端到端 smoke test：创建临时 Git repository、pin、inspect 平台 launcher、重复 pin、unpin，并保证 teardown 无残留
- [ ] 7.3 完成 README 的三平台前置条件、portable 安装、命令示例、入口位置、名称冲突、稳定版 VS Code 限制、macOS 未签名提示和 V1 非目标文档
- [ ] 7.4 推送端到端与文档小步，确认 fmt、clippy、全部测试和三平台 release build 均绿色

## 8. Portable Packaging

- [ ] 8.1 建立平台/架构 staging 与命名逻辑，校验 tag semver、Cargo version、包名和文档版本一致，不一致时在上传前失败
- [ ] 8.2 为 Windows 包装两个 `.exe`，为 Linux/macOS 包装两个正式 binary，统一加入 README/MIT LICENSE、同名顶层目录，并保留 Unix 可执行权限
- [ ] 8.3 添加包复验测试：解压 ZIP 并校验精确内容、目录名、版本、权限、binary 可运行性，以及每个 ZIP 的 SHA-256 摘要
- [ ] 8.4 在三个原生 OS 的 runner-native/x86_64 矩阵生成仅供 CI 下载的 package artifacts；逐项验证可用的 arm64 原生平台构建，无法可靠构建/测试的组合明确不加入 V1 支持矩阵

## 9. 上游扫描与公开 Release

- [ ] 9.1 审阅完整直接/传递 dependency 清单，移除非必要依赖并记录每项上游许可证；保持项目最终许可证为 MIT
- [ ] 9.2 配置固定版本的 Rust dependency 许可证和 advisory 扫描策略，使不兼容/未知未审核许可证及策略禁止的安全公告阻断 release
- [ ] 9.3 添加 tag-triggered release workflow：先完成所有必需平台 build/test/package、合规门禁和 checksum，再由单独 publish job 原子式创建 GitHub Release，任一矩阵失败不发布部分版本
- [ ] 9.4 以非公开 dry run 验证完整 release 流程与失败路径，复核 ZIP、SHA-256、MIT LICENSE、三平台安装说明和无管理员权限要求后，才允许创建首个公开 V1 tag
