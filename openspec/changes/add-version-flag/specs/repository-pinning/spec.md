## MODIFIED Requirements

### Requirement: Git 外部命令接口
系统 SHALL 提供名为 `git-pin` 和 `git-unpin` 的独立可执行文件，使 Git 可将其分别作为 `git pin` 和 `git unpin` 调用。`git pin` SHALL 接受零个或一个位置参数，或单独接受 `--help`、`-h`、`--list`、`--prune`、`--version`、`-v` 之一；`git unpin` SHALL 保持接受零个或一个位置参数。未知选项、互斥模式与位置参数的组合或多余参数 SHALL 输出用法错误并以非零状态退出。由于 Git 前端会在外部命令分派前将 `git pin --help` 转换为 Git 文档查询，系统 SHALL 将 `git pin -h` 与直接调用 `git-pin --help` 作为可执行文件提供的完整文本帮助入口，并 SHALL 在帮助文本和用户文档中说明该限制及替代入口。

#### Scenario: Git 分派 pin 命令
- **WHEN** 两个可执行文件所在目录已加入 `PATH`，用户运行 `git pin`
- **THEN** Git 调用 `git-pin`，且命令以当前工作目录作为输入

#### Scenario: 通过 Git 显示完整帮助
- **WHEN** 用户运行 `git pin -h`
- **THEN** 命令向标准输出显示包含 `git pin [path]`、`git pin --list`、`git pin --prune`、`git pin --version`、`pin.ide` 及帮助入口限制的说明，以零状态退出且不修改任何启动器

#### Scenario: 直接显示完整帮助
- **WHEN** 用户运行 `git-pin --help` 或 `git-pin -h`
- **THEN** 可执行文件向标准输出显示同一份完整帮助，以零状态退出且不修改任何启动器

#### Scenario: Git 占用双横线帮助
- **WHEN** 用户运行 `git pin --help`
- **THEN** 用户文档已明确说明 Git 会在调用 `git-pin` 前按自身文档机制处理该调用，并引导使用 `git pin -h` 或 `git-pin --help`

#### Scenario: 拒绝 V1 范围外参数
- **WHEN** 用户传入未知选项、多于一个位置参数，或将 `--list`、`--prune`、`--version`、`--help` 与位置参数或其他模式组合
- **THEN** 命令在标准错误输出简明用法信息，以非零状态退出且不修改任何启动器

## ADDED Requirements

### Requirement: 版本查询
`git pin --version` 与 `git pin -v` SHALL 向标准输出打印当前可执行文件的版本号并以零状态退出。版本号 MUST 与构建该可执行文件时使用的包版本一致，不得由源码中的独立字面量维护。版本查询 SHALL 是无副作用模式：不读取 Git 配置、不解析 repository、不访问平台启动器目录，也不创建、修改或删除任何启动器。版本查询 SHALL 在 Git 外部命令分派下可用，即 `git pin --version` 与 `git pin -v` 均直接由 `git-pin` 处理。

#### Scenario: 通过 Git 查询版本
- **WHEN** 用户运行 `git pin --version`
- **THEN** 命令向标准输出打印包含当前版本号的单行文本，以零状态退出，且不向标准错误输出内容

#### Scenario: 短选项等价
- **WHEN** 用户运行 `git pin -v`
- **THEN** 输出与 `git pin --version` 完全相同的版本信息，并以零状态退出

#### Scenario: 版本与构建版本一致
- **WHEN** 可执行文件由版本号为 `X.Y.Z` 的包构建
- **THEN** 版本查询输出包含 `X.Y.Z`，且该值随包版本变化而自动变化

#### Scenario: 版本查询无副作用
- **WHEN** 用户在非 Git repository 目录、或平台启动器目录不可写的环境中运行 `git pin --version`
- **THEN** 命令仍以零状态退出并打印版本号，不产生任何错误，也不创建或删除启动器

#### Scenario: 版本选项不可与其他模式组合
- **WHEN** 用户运行 `git pin --version --list`、`git pin -v path/to/repo` 或 `git pin --version -h`
- **THEN** 命令在标准错误输出用法信息，以非零状态退出，且不打印版本号也不修改任何启动器
