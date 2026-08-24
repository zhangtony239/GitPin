# repository-pinning Specification

## Purpose

定义 `git pin` 与 `git unpin` 在三大桌面平台上的统一仓库识别、入口管理、冲突处理和启动行为，使用户能从系统启动界面可靠地用 Visual Studio Code 打开已注册仓库。

## Requirements

### Requirement: Git 外部命令接口
系统 SHALL 提供名为 `git-pin` 和 `git-unpin` 的独立可执行文件，使 Git 可将其分别作为 `git pin` 和 `git unpin` 调用。每个命令 SHALL 接受零个或一个位置参数；不受支持的参数或多余参数 SHALL 输出用法错误并以非零状态退出。

#### Scenario: Git 分派 pin 命令
- **WHEN** 两个可执行文件所在目录已加入 `PATH`，用户运行 `git pin`
- **THEN** Git 调用 `git-pin`，且命令以当前工作目录作为输入

#### Scenario: 拒绝 V1 范围外参数
- **WHEN** 用户向任一命令传入多个位置参数或 `--name`、`--list`、`--prune`、`--all` 等不支持的选项
- **THEN** 命令在标准错误输出简明用法信息，并以非零状态退出且不修改任何启动器

### Requirement: Repository root 发现与规范化
对于无参数调用，系统 SHALL 从当前工作目录发现 Git repository；对于路径参数，系统 SHALL 从该路径发现 repository。系统 MUST 使用 Git 所认定的顶层工作树作为规范 repository root，而不是直接使用调用目录或传入的子目录。无法执行 Git、输入不存在、输入不属于含工作树的 Git repository，或无法获得绝对 root 时 SHALL 明确失败且不修改启动器。

#### Scenario: 从仓库子目录 pin
- **WHEN** 用户在 repository 的任意子目录运行 `git pin`
- **THEN** 系统注册该 repository 的绝对顶层工作树路径

#### Scenario: 从给定子目录 pin
- **WHEN** 用户运行 `git pin path/to/repo/subdirectory` 且该路径属于一个 Git 工作树
- **THEN** 系统注册 Git 返回的顶层工作树，而不是给定子目录

#### Scenario: 非仓库输入
- **WHEN** 用户在非 Git repository 中运行无参数命令，或向需要 repository 的调用传入非仓库路径
- **THEN** 系统输出可诊断错误、以非零状态退出，并且不创建或删除启动器

### Requirement: Repository 默认名称
系统 SHALL 以规范 repository root 的最后一个路径组件作为 repository name，并原样保留用户可见名称。若该名称为空或无法在当前平台安全表示为启动器名称，系统 SHALL 明确失败，而不是静默改名。

#### Scenario: 从 root 生成名称
- **WHEN** 规范 repository root 为平台路径中最后一个组件名为 `personal-site` 的目录
- **THEN** 系统使用 `personal-site` 作为启动器显示名称和查找名称

#### Scenario: 名称不可安全表示
- **WHEN** repository basename 无法安全映射为当前平台启动器名称
- **THEN** pin 以非零状态退出、解释名称限制，且不创建替代名称的启动器

### Requirement: Pin 创建或确认启动器
`git pin [path]` SHALL 为规范 repository root 创建当前平台的单个启动器。创建成功后，从该启动器启动 SHALL 在 Visual Studio Code 中打开该 root。若同名启动器已指向同一规范 root，pin SHALL 成功且保持单个有效启动器；若已指向不同 root，pin MUST 拒绝覆盖并报告现有目标。系统 SHALL 以原子方式提交启动器，失败时不得留下可被误认为有效注册的半成品。

#### Scenario: 首次 pin
- **WHEN** 尚无同名 git-pin 启动器，用户 pin 一个有效 repository
- **THEN** 系统创建一个指向其规范 root 的平台启动器并以成功状态退出

#### Scenario: 重复 pin 同一仓库
- **WHEN** 同名启动器已经指向同一规范 root，用户再次 pin 该 repository
- **THEN** 命令成功结束，且系统中仍只有一个对应启动器

#### Scenario: 同 basename 冲突
- **WHEN** 同名启动器已经指向另一个规范 root，用户 pin 当前 repository
- **THEN** 命令以非零状态退出，报告名称及已注册 root，并保持现有启动器不变

#### Scenario: Visual Studio Code 不可用
- **WHEN** 平台无法找到可用于启动仓库的 Visual Studio Code 安装
- **THEN** pin 明确失败且不提交无效启动器

### Requirement: Unpin 支持当前仓库、路径和名称
`git unpin` SHALL 根据当前目录的规范 repository root 删除对应启动器。`git unpin <argument>` 在参数指向现存文件系统路径时 SHALL 按 repository 路径解析并删除；否则 SHALL 将参数作为精确 repository name 删除。仅允许删除由 git-pin 管理且与所求 root 或名称匹配的启动器。目标不存在时 unpin SHALL 作为幂等操作成功返回。

#### Scenario: Unpin 当前仓库
- **WHEN** 用户在已 pin repository 的子目录运行 `git unpin`
- **THEN** 系统删除指向该 repository 规范 root 的启动器

#### Scenario: 按路径 unpin
- **WHEN** 用户向 `git unpin` 传入一个现存 repository 或其子目录路径
- **THEN** 系统解析规范 root 并删除名称和目标均匹配的启动器

#### Scenario: 按名称 unpin
- **WHEN** 用户运行 `git unpin reponame`，且 `reponame` 不解析为现存路径
- **THEN** 系统删除精确名为 `reponame` 的受管启动器而无需访问原 repository

#### Scenario: Unpin 不存在的注册
- **WHEN** 没有与所求 repository 或名称匹配的受管启动器
- **THEN** 命令不删除其他文件并以成功状态退出

#### Scenario: 拒绝删除不匹配目标
- **WHEN** 按路径 unpin 找到同名启动器但其记录的规范 root 与请求 root 不同
- **THEN** 命令报告冲突、以非零状态退出并保留该启动器

### Requirement: Windows 启动器行为
在 Windows 上，系统 SHALL 将受管启动器保存为 `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<repo-name>.lnk`。快捷方式 SHALL 直接启动已解析的 Visual Studio Code 可执行文件，以规范 root 作为参数和工作目录，并使用 Visual Studio Code 图标。路径比较 SHALL 遵循 Windows 的不区分大小写语义。

#### Scenario: 创建 Windows 快捷方式
- **WHEN** 用户在 Windows 上成功 pin 名为 `foo` 的 repository
- **THEN** `.pinned_repo\foo.lnk` 存在，且其目标、参数、工作目录和图标可使开始菜单入口在 Visual Studio Code 中打开规范 root

#### Scenario: Windows 路径包含空格或非 ASCII 字符
- **WHEN** repository root 包含空格、引号可表示字符或非 ASCII 字符
- **THEN** 快捷方式保留准确路径，启动时将整个 root 作为一个参数传递且不执行路径中的内容

### Requirement: Linux 启动器行为
在 Linux 上，系统 SHALL 将受管启动器保存为 `${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<repo-name>.desktop`。Desktop Entry SHALL 至少声明 `Type=Application`、repository 显示名称、Visual Studio Code 图标、`Terminal=false`，以及安全编码的启动命令和规范 root。成功 pin 后文件 SHALL 具备桌面环境启动所需权限，并在可用时刷新 desktop entry 数据库；刷新工具缺失不得导致 pin 失败。

#### Scenario: 创建 Linux desktop entry
- **WHEN** 用户在 Linux 上成功 pin 名为 `foo` 的 repository 且未设置 `XDG_DATA_HOME`
- **THEN** `~/.local/share/applications/git-pin-foo.desktop` 是有效、可启动的 Desktop Entry，并在 Visual Studio Code 中打开规范 root

#### Scenario: 遵循 XDG_DATA_HOME
- **WHEN** 用户设置了绝对路径形式的 `XDG_DATA_HOME` 后 pin repository
- **THEN** 系统在该目录的 `applications` 子目录创建启动器，而不写入默认目录

#### Scenario: Linux 路径安全编码
- **WHEN** repository root 包含空格、desktop-entry 字段特殊字符或非 ASCII 字符
- **THEN** 启动器按 Desktop Entry 规范编码路径，并且启动时不发生参数拆分或命令注入

### Requirement: macOS 启动器行为
在 macOS 上，系统 SHALL 将每个受管启动器保存为 `$HOME/Applications/Git Pin/<repo-name>.app` 标准应用 bundle。Bundle SHALL 具有稳定且不会与其他 repository 冲突的标识、可执行启动入口和描述性元数据，并 SHALL 通过系统应用启动机制在 Visual Studio Code 中打开规范 root，使入口可从 Finder 打开并可被 Spotlight 索引。系统不得修改系统级 `/Applications`，也不得要求管理员权限。

#### Scenario: 创建 macOS app bundle
- **WHEN** 用户在 macOS 上成功 pin 名为 `foo` 的 repository
- **THEN** `$HOME/Applications/Git Pin/foo.app` 是可由 Finder 启动的有效应用 bundle，启动后 Visual Studio Code 打开规范 root

#### Scenario: macOS 路径安全传递
- **WHEN** repository root 包含空格、shell 元字符或非 ASCII 字符
- **THEN** bundle 启动入口将准确 root 作为数据传给系统应用启动机制，不将路径解释为可执行 shell 内容

#### Scenario: macOS 用户级安装
- **WHEN** 非管理员用户 pin repository
- **THEN** 系统仅写入该用户的 Applications 目录并成功创建入口，不请求提权

### Requirement: 错误与操作结果可诊断
每个命令 SHALL 以零状态表示操作成功或幂等无变化，以非零状态表示用法、repository、冲突、依赖或 I/O 失败。错误 SHALL 写入标准错误并包含失败动作及相关路径或名称；系统不得输出 Rust panic 或无上下文的底层错误作为正常失败界面。

#### Scenario: 启动器目录不可写
- **WHEN** pin 或 unpin 无权修改平台启动器目录
- **THEN** 命令以非零状态退出，并报告动作、目标位置及可理解的失败原因

#### Scenario: 成功命令适合脚本调用
- **WHEN** pin 或 unpin 完成所请求状态且没有错误
- **THEN** 命令以零状态退出，不向标准错误输出失败信息
