## Why

Git Pin 当前把 Visual Studio Code 作为固定启动目标，无法复用 Cursor 等同样支持以 `ide path/to/repo` 形式启动仓库的编辑器。同时，Git 会在分派外部子命令前把 `git pin --help` 改写为文档查询，现有帮助承诺与 Git 的实际行为不符，需要在 v1.2 明确可用入口。

## What Changes

- 采用 Git config 作为 Git Pin 的配置系统，所有项目配置键统一使用 `pin.` 前缀。
- 新增字符串配置 `pin.ide`，默认值为 `code`；其值是 PATH 中的 executable 名称或可执行文件路径，不是 shell 命令模板。
- `git pin` 按 Git 正常的命令行、仓库、全局和系统配置优先级读取有效 `pin.ide`，解析可执行文件并把该启动目标固化到新建 launcher 中。
- 允许任何支持以单个 repository root 参数启动的 IDE CLI；产品文案和行为契约不再绑定 Visual Studio Code。
- 配置变更只影响之后新建的 launcher；既有 launcher 不动态读取配置，重复 pin 同一仓库仍保持既有幂等语义而不暗中改写。
- 明确 Git 会提前占用 `git pin --help`；对外完整帮助入口为 `git pin -h` 或 `git-pin --help`，帮助文本本身也说明该限制。
- 同步更新中英文 README，说明 v1.2 配置、IDE 启动契约、配置作用域/优先级、固化语义与帮助入口。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `repository-pinning`: 将固定 Visual Studio Code 启动目标改为由 Git config 的 `pin.ide` 选择并在 pin 时固化的通用 IDE CLI，同时修正外部 Git 子命令的帮助入口契约。

## Impact

- CLI 帮助文本、Git 外部命令跨进程测试及中英文 README 需要更新。
- repository pin 编排需要读取并校验 Git config，平台后端接口与受管 launcher 元数据需要携带已解析 IDE executable。
- Windows `.lnk`、Linux Desktop Entry 与 macOS app bundle 的创建、解析、枚举和测试 fixture 需要解除对 Visual Studio Code 名称及路径的假设。
- 不新增配置文件、配置解析依赖或安装器；继续依赖用户现有 Git，并保持 portable 发布与用户级 launcher 存储边界。
