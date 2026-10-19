## MODIFIED Requirements

### Requirement: Git 外部命令接口
系统 SHALL 提供名为 `git-pin` 和 `git-unpin` 的独立可执行文件，使 Git 可将其分别作为 `git pin` 和 `git unpin` 调用。`git pin` SHALL 接受零个或一个位置参数，或单独接受 `--help`、`-h`、`--list`、`--prune` 之一；`git unpin` SHALL 保持接受零个或一个位置参数。未知选项、互斥模式与位置参数的组合或多余参数 SHALL 输出用法错误并以非零状态退出。

#### Scenario: Git 分派 pin 命令
- **WHEN** 两个可执行文件所在目录已加入 `PATH`，用户运行 `git pin`
- **THEN** Git 调用 `git-pin`，且命令以当前工作目录作为输入

#### Scenario: 显示完整帮助
- **WHEN** 用户运行 `git pin --help` 或 `git pin -h`
- **THEN** 命令向标准输出显示包含 `git pin [path]`、`git pin --list` 与 `git pin --prune` 的帮助及各模式说明，以零状态退出且不修改任何启动器

#### Scenario: 拒绝 V1 范围外参数
- **WHEN** 用户传入未知选项、多于一个位置参数，或将 `--list`、`--prune`、`--help` 与位置参数或其他模式组合
- **THEN** 命令在标准错误输出简明用法信息，以非零状态退出且不修改任何启动器

### Requirement: 受管启动器列表与有效性自检
`git pin --list` SHALL 枚举当前平台存储位置中全部可识别为 Git Pin 管理的启动器，并为每项输出 repository name、记录的 repository root 和有效性状态。有效性自检 SHALL 验证记录的 root 当前存在、是目录且仍属于一个含工作树的 Git repository，其 Git 顶层工作树与记录的 root 按平台路径语义相同；检查 SHALL NOT 因 Visual Studio Code 当前不可用而将启动器判为无效。输出顺序 SHALL 按 repository name 确定性排序。命令 SHALL NOT 修改任何启动器。

#### Scenario: 列出有效快捷方式
- **WHEN** 受管启动器记录的 root 存在，并且该 root 仍是对应 Git 工作树的规范顶层目录
- **THEN** `git pin --list` 输出其名称、root 和有效状态

#### Scenario: 标记不存在的 root
- **WHEN** 受管启动器记录的 root 已不存在或不再是目录
- **THEN** `git pin --list` 保留该项并将其报告为无效，而不删除启动器

#### Scenario: 标记不再匹配的 repository
- **WHEN** 记录的 root 存在但不再属于含工作树的 Git repository，或 Git 返回的顶层工作树与记录值不相同
- **THEN** `git pin --list` 将该项报告为无效并提供可诊断原因

#### Scenario: 忽略非受管入口
- **WHEN** 平台启动器目录还包含无法识别为 Git Pin 所管理的文件或应用入口
- **THEN** `git pin --list` 不将这些入口作为 repository 项输出且不修改它们

#### Scenario: 空列表
- **WHEN** 当前平台没有任何可识别的受管启动器
- **THEN** `git pin --list` 明确报告没有 pinned repository 并以零状态退出

#### Scenario: 确定性排序
- **WHEN** 当前平台存在多个受管启动器
- **THEN** 每次 `git pin --list` 均按 repository name 排序输出各项

#### Scenario: Code 缺失不影响 repository 有效性
- **WHEN** 受管启动器记录的 repository root 有效但系统当前无法找到 Visual Studio Code
- **THEN** `git pin --list` 仍将该启动器报告为有效

### Requirement: 僵尸启动器批量清理
`git pin --prune` SHALL 枚举并自检当前平台全部可识别的受管启动器，删除记录的 repository root 已不存在、不是目录、不再属于含工作树的 Git repository，或其 Git 顶层工作树与记录值不匹配的启动器。命令 MUST 仅删除在本次扫描中识别并验证为僵尸的 Git Pin 受管启动器，SHALL 保留有效启动器和非受管入口，并 SHALL 为每个成功删除项输出其名称与记录的 root。Visual Studio Code 不可用 SHALL NOT 构成清理条件。

#### Scenario: 清理全部僵尸快捷方式
- **WHEN** 当前平台存在多个因 repository root 不存在或不再有效而成为僵尸的受管启动器
- **THEN** `git pin --prune` 删除扫描到的全部僵尸启动器并报告每个删除项

#### Scenario: 保留有效快捷方式
- **WHEN** 扫描结果同时包含有效与僵尸受管启动器
- **THEN** `git pin --prune` 只删除僵尸启动器并保持有效启动器不变

#### Scenario: 保留非受管入口
- **WHEN** 平台启动器目录包含非 Git Pin 管理的文件或应用入口
- **THEN** `git pin --prune` 不删除或修改这些入口

#### Scenario: 无需清理
- **WHEN** 扫描中没有发现僵尸受管启动器
- **THEN** `git pin --prune` 明确报告无需清理并以零状态退出

#### Scenario: Code 缺失不触发清理
- **WHEN** 受管启动器的 repository root 有效但 Visual Studio Code 当前不可用
- **THEN** `git pin --prune` 保留该启动器

### Requirement: 错误与操作结果可诊断
每个命令 SHALL 以零状态表示操作成功或幂等无变化，以非零状态表示用法、repository、冲突、依赖、枚举、自检、删除或 I/O 失败。错误 SHALL 写入标准错误并包含失败动作及相关路径或名称；系统不得输出 Rust panic 或无上下文的底层错误作为正常失败界面。`--list` 或 `--prune` 遇到单项无法解析、检查或删除时 SHALL 继续处理其他候选项，报告所有已发现失败，并最终以非零状态退出；`--prune` SHALL NOT 回滚已成功完成的其他删除。

#### Scenario: 启动器目录不可写
- **WHEN** pin、unpin 或 prune 无权修改平台启动器目录
- **THEN** 命令以非零状态退出，并报告动作、目标位置及可理解的失败原因

#### Scenario: 成功命令适合脚本调用
- **WHEN** pin、unpin、list 或 prune 完成所请求状态且没有错误
- **THEN** 命令以零状态退出，不向标准错误输出失败信息

#### Scenario: 列表部分检查失败
- **WHEN** `git pin --list` 无法读取或自检某一候选受管启动器，但仍可处理其他候选项
- **THEN** 命令继续输出其他项，向标准错误报告失败项及原因，并最终以非零状态退出

#### Scenario: 清理部分删除失败
- **WHEN** `git pin --prune` 成功删除部分僵尸启动器但另一个僵尸启动器删除失败
- **THEN** 命令保留已完成的删除，继续处理其余项，报告失败项并最终以非零状态退出
