# Spec Delta

## MODIFIED Requirements

### Requirement: 受管启动器列表与有效性自检
`git pin --list` SHALL 枚举当前平台存储位置中全部可识别为 Git Pin 管理的启动器，并为每项输出 repository name、记录的 repository root 和有效性状态。有效性自检 SHALL 验证记录的 root 当前存在、是目录且仍属于一个含工作树的 Git repository，其 Git 顶层工作树与记录的 root 按平台路径语义相同；检查 SHALL NOT 因 Visual Studio Code 当前不可用而将启动器判为无效。输出顺序 SHALL 按 repository name 确定性排序。命令 SHALL NOT 修改任何启动器。列表 SHALL 以对齐的列呈现，列宽按本次输出的实际内容计算，使同一列在所有行中起始于相同的显示列位置；对齐 SHALL 在 name 或 root 含非 ASCII 字符时仍成立。

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

#### Scenario: 列按内容对齐
- **WHEN** 当前平台存在多个受管启动器，且其 repository name 与 root 长度各不相同
- **THEN** `git pin --list` 输出的每一行中，root 列与状态列均起始于相同的显示列位置，且列间以空格填充而非依赖固定制表位

#### Scenario: 长名称与短名称混合
- **WHEN** 受管启动器的 repository name 长度跨越多个制表位宽度（例如同时存在 3 字符与 20 字符的名称）
- **THEN** 所有行的 root 列仍对齐，短名称行以空格补齐到该列起始位置

#### Scenario: 非 ASCII 名称对齐
- **WHEN** 受管启动器的 repository name 或 root 包含非 ASCII 字符
- **THEN** 列对齐按字符显示宽度计算，后续列不因多字节字符而错位

#### Scenario: 无效项与有效项混合对齐
- **WHEN** 同一次 `git pin --list` 输出同时包含有效项与带原因说明的无效项
- **THEN** 两类行的 name 列与 root 列对齐一致，无效原因仅出现在状态列内
