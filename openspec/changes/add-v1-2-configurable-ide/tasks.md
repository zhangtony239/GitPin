## 1. 配置读取与 executable 解析

- [ ] 1.1 增加 `pin.ide` 配置读取模块，通过 Git 配置接口取得当前调用的有效字符串值，未设置时回退到 `code`，并为 Git 读取失败提供带键名的诊断
- [ ] 1.2 实现原子 IDE executable 解析：无目录组件时搜索 PATH，含目录组件时按单个路径解析并绝对化，拒绝空值、目录、缺失目标及不能作为单个 executable 的命令模板
- [ ] 1.3 增加配置与解析单元测试，覆盖默认值、系统/全局/仓库/命令行优先级、Cursor 等命令名、绝对/相对/含空格路径、缺失 executable 和 shell/参数模板拒绝
- [ ] 1.4 将配置读取限制在 Pin 动作，验证 Help、List、Prune 与 Unpin 不会因 `pin.ide` 无效或 IDE 缺失而失败

## 2. 共享 launcher 契约

- [ ] 2.1 扩展 launcher 创建输入和受管 launcher 元数据以携带已解析 IDE executable，并更新 fake backend、应用层调用方及现有 fixture
- [ ] 2.2 保持同 root 重复 pin 的既有幂等语义，增加当前 `pin.ide` 改变后不改写已固化 launcher 的应用层测试
- [ ] 2.3 确保 list/prune 的 repository 有效性判断不检查已固化 IDE 是否仍存在，并增加 IDE 被移动后仍保留有效 repository launcher 的测试

## 3. 三平台通用 IDE launcher

- [ ] 3.1 更新 Windows backend，使 `.lnk` 固化所选 IDE executable、以 repository root 作为单个参数和工作目录、使用 executable 图标，并能从新旧 launcher 恢复该目标
- [ ] 3.2 更新 Linux backend，使 Desktop Entry 安全编码所选 IDE executable 与单个 root 参数、使用可用 IDE 图标，并保持旧 Code entry 的 inspect/enumerate/unpin 兼容
- [ ] 3.3 更新 macOS bundle metadata 与内嵌启动路径，使其无 shell 地直接执行固化 IDE CLI 和单个 root 参数，不再依赖 `open -a "Visual Studio Code"`
- [ ] 3.4 为 Windows、Linux、macOS 增加隔离 fixture 测试，覆盖自定义 CLI、executable/root 中的空格和非 ASCII 字符、安全参数边界及旧 v1.0/v1.1 Code launcher 兼容

## 4. 帮助契约与真实 Git 分派

- [ ] 4.1 更新完整帮助文本，说明 `pin.ide`、默认 `code`、通用 `ide path/to/repo` 契约，以及 Git 占用 `git pin --help` 后应使用 `git pin -h` 或 `git-pin --help`
- [ ] 4.2 扩展跨进程测试，真实运行 `git pin -h` 并直接运行 `git-pin --help`/`-h`，验证同一完整 stdout、零退出且无 launcher 副作用
- [ ] 4.3 增加真实 Git 分派配置测试，验证 `git -c pin.ide=<fixture> pin` 与仓库/全局配置优先级能到达外部命令并固化预期 executable

## 5. 文档、兼容与发布验证

- [ ] 5.1 更新英文 README 的 v1.2 定位、帮助入口、`pin.ide` 配置示例、作用域优先级、受支持 IDE CLI 契约、固化语义和切换既有 launcher 流程
- [ ] 5.2 同步更新中文 README，删除产品与 Visual Studio Code 绑定的表述，并与英文文档保持相同配置及限制说明
- [ ] 5.3 运行格式化、静态检查、完整测试与 OpenSpec 严格验证，并通过 Windows、Linux、macOS CI 确认自定义 IDE launcher 和旧 launcher 兼容
- [ ] 5.4 将 Cargo package/lockfile 版本更新到 `1.2.0`，通过单击 release dry run 或正式流程确认版本派生、三平台 portable 包及帮助/配置 smoke test
