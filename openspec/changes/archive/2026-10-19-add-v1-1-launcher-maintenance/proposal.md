## Why

Git Pin v1.0 只支持创建和删除单个仓库入口，缺少可发现的命令帮助、全量状态查看以及批量清理能力。v1.1 需要让用户能够理解可用选项、检查所有受管快捷方式，并一次性移除仓库已不存在的僵尸入口。

## What Changes

- 完善 `git pin --help`（并支持 `-h`），清晰说明 pin、list、prune 等调用方式，正常显示帮助而不执行修改。
- 新增/完善 `git pin --list`，枚举当前平台全部由 Git Pin 管理的快捷方式，输出其名称、记录的 repository root 与有效性自检结果。
- 新增/完善 `git pin --prune`，扫描全部受管快捷方式并删除记录的 repository root 已不存在或已不再是 Git 工作树的僵尸快捷方式，同时保留有效快捷方式与非 Git Pin 管理的入口。
- 为 list/prune 的部分读取、校验或删除失败定义可诊断的输出与非零退出状态；无可列出或清理项时保持成功且幂等。
- 不处理因 Visual Studio Code 缺失而失效的快捷方式；该情况由系统自行移除或忽略。
- 保持既有 `git pin [path]` 与 `git unpin [path|name]` 行为兼容。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `repository-pinning`: 扩展 Git 外部命令接口、受管启动器枚举与校验、批量清理以及对应的结果诊断要求。

## Impact

- 受影响代码主要包括 CLI 参数解析与帮助输出、应用层操作分派、跨平台启动器后端接口及 Windows/Linux/macOS 的受管启动器发现和元数据读取实现。
- 需要扩展单元测试和跨进程集成测试，覆盖帮助、空列表、有效/无效/损坏/外部快捷方式混合列表、批量清理、幂等与部分失败。
- 不引入新的用户级配置、外部服务或运行时依赖；平台快捷方式存储位置保持不变。
