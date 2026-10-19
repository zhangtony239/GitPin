# repository-pinning Specification

## Purpose

定义 `git pin` 与 `git unpin` 在三大桌面平台上的统一仓库识别、入口管理、冲突处理和启动行为，使用户能从系统启动界面可靠地用 Visual Studio Code 打开已注册仓库。

## Requirements

### Requirement: Git 外部命令接口
系统 SHALL 提供名为 `git-pin` 和 `git-unpin` 的独立可执行文件，使 Git 可将其分别作为 `git pin` 和 `git unpin` 调用。`git pin` SHALL 接受零个或一个位置参数，或单独接受 `--help`、`-h`、`--list`、`--prune` 之一；`git unpin` SHALL 保持接受零个或一个位置参数。未知选项、互斥模式与位置参数的组合或多余参数 SHALL 输出用法错误并以非零状态退出。由于 Git 前端会在外部命令分派前将 `git pin --help` 转换为 Git 文档查询，系统 SHALL 将 `git pin -h` 与直接调用 `git-pin --help` 作为可执行文件提供的完整文本帮助入口，并 SHALL 在帮助文本和用户文档中说明该限制及替代入口。

#### Scenario: Git 分派 pin 命令
- **WHEN** 两个可执行文件所在目录已加入 `PATH`，用户运行 `git pin`
- **THEN** Git 调用 `git-pin`，且命令以当前工作目录作为输入

#### Scenario: 通过 Git 显示完整帮助
- **WHEN** 用户运行 `git pin -h`
- **THEN** 命令向标准输出显示包含 `git pin [path]`、`git pin --list`、`git pin --prune`、`pin.ide` 及帮助入口限制的说明，以零状态退出且不修改任何启动器

#### Scenario: 直接显示完整帮助
- **WHEN** 用户运行 `git-pin --help` 或 `git-pin -h`
- **THEN** 可执行文件向标准输出显示同一份完整帮助，以零状态退出且不修改任何启动器

#### Scenario: Git 占用双横线帮助
- **WHEN** 用户运行 `git pin --help`
- **THEN** 用户文档已明确说明 Git 会在调用 `git-pin` 前按自身文档机制处理该调用，并引导使用 `git pin -h` 或 `git-pin --help`

#### Scenario: 拒绝 V1 范围外参数
- **WHEN** 用户传入未知选项、多于一个位置参数，或将 `--list`、`--prune`、`--help` 与位置参数或其他模式组合
- **THEN** 命令在标准错误输出简明用法信息，以非零状态退出且不修改任何启动器

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

### Requirement: Git config 配置命名与 IDE 选择
Git Pin 的公开配置键 SHALL 使用 `pin.` 前缀。系统 SHALL 支持字符串配置 `pin.ide`，未配置时 SHALL 使用 `code`。该值 MUST 表示一个 executable 名称或可执行文件路径，系统不得将其解析为 shell 命令、参数列表或命令模板。`git pin` SHALL 通过 Git 配置接口读取调用上下文中的有效值，使命令行、仓库、全局和系统配置遵循 Git 自身的正常优先级。所选 IDE CLI MUST 支持将一个 repository root 作为位置参数接收。

#### Scenario: 使用默认 IDE
- **WHEN** 当前 Git 配置上下文没有设置 `pin.ide`
- **THEN** `git pin` 使用 `code` 作为待解析的 IDE executable

#### Scenario: 使用 Cursor 命令
- **WHEN** 用户设置有效配置 `pin.ide=cursor` 后 pin repository
- **THEN** 系统从 PATH 解析 `cursor` 并创建以 Cursor CLI 打开该 repository root 的启动器

#### Scenario: 遵循 Git 配置优先级
- **WHEN** 系统、全局、仓库或命令行配置在多个作用域提供不同的 `pin.ide` 值
- **THEN** `git pin` 使用 Git 对本次调用解析出的有效配置值

#### Scenario: 使用 executable 路径
- **WHEN** `pin.ide` 是包含空格的有效 IDE executable 路径
- **THEN** 系统将完整路径作为单个 executable 值解析，不进行 shell 拆词

#### Scenario: 拒绝命令模板
- **WHEN** `pin.ide` 包含试图附加参数或 shell 操作符的命令模板而不能作为单个 executable 解析
- **THEN** pin 明确失败且不创建或修改启动器

#### Scenario: 配置只影响新启动器
- **WHEN** 用户修改 `pin.ide`，而某个 repository 已有受管启动器
- **THEN** 既有启动器继续使用创建时固化的 IDE executable，且系统不因配置变化自动改写它

### Requirement: Pin 创建或确认启动器
`git pin [path]` SHALL 为规范 repository root 创建当前平台的单个启动器。创建前，系统 SHALL 读取有效 `pin.ide`、解析对应 IDE executable，并将解析结果固化到启动器。从该启动器启动 SHALL 以规范 root 作为单个位置参数调用该 IDE。若同名受管启动器已指向同一规范 root，pin SHALL 成功且保持既有启动器及其已固化 IDE 不变；若已指向不同 root，pin MUST 拒绝覆盖并报告现有目标。系统 SHALL 以原子方式提交启动器，失败时不得留下可被误认为有效注册的半成品。

#### Scenario: 首次 pin
- **WHEN** 尚无同名 Git Pin 启动器，用户以可解析的 `pin.ide` pin 一个有效 repository
- **THEN** 系统创建一个固化该 IDE executable、并以其打开规范 root 的平台启动器，以成功状态退出

#### Scenario: 重复 pin 同一仓库
- **WHEN** 同名启动器已经指向同一规范 root，用户在 `pin.ide` 相同或已变化后再次 pin 该 repository
- **THEN** 命令成功结束，系统中仍只有一个对应启动器，且既有启动器目标不被暗中改写

#### Scenario: 同 basename 冲突
- **WHEN** 同名启动器已经指向另一个规范 root，用户 pin 当前 repository
- **THEN** 命令以非零状态退出，报告名称及已注册 root，并保持现有启动器不变

#### Scenario: Visual Studio Code 不可用
- **WHEN** 平台无法将有效 `pin.ide` 值解析为可执行的 IDE CLI
- **THEN** pin 明确失败并报告配置值，且不提交无效启动器

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

### Requirement: Windows 启动器行为
在 Windows 上，系统 SHALL 将受管启动器保存为 `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<repo-name>.lnk`。快捷方式 SHALL 直接启动 pin 时解析并固化的 IDE executable，以规范 root 作为单个参数和工作目录，并使用该 executable 可用的图标。未配置 `pin.ide`、因而选择默认 `code` 时，快捷方式 MUST 以 Visual Studio Code GUI executable 为目标，不得把 `.cmd` 或 `.bat` CLI shim 固化为目标，且从开始菜单启动 repository 时不得出现伴随的可见控制台窗口。路径比较 SHALL 遵循 Windows 的不区分大小写语义。

#### Scenario: 创建 Windows 快捷方式
- **WHEN** 用户在 Windows 上以有效 `pin.ide` 成功 pin 名为 `foo` 的 repository
- **THEN** `.pinned_repo\foo.lnk` 存在，且其目标、参数和工作目录可使开始菜单入口在所选 IDE 中打开规范 root

#### Scenario: Windows 路径包含空格或非 ASCII 字符
- **WHEN** IDE executable 或 repository root 包含空格、引号可表示字符或非 ASCII 字符
- **THEN** 快捷方式保留准确路径，启动时将整个 root 作为一个参数传递且不执行路径中的内容

#### Scenario: Windows 默认 Code 使用 GUI 入口
- **WHEN** Windows 用户未配置 `pin.ide`，且 PATH 中的 `code` 命令由 Visual Studio Code 的 `code.cmd` CLI shim 提供
- **THEN** 系统创建以对应有效 `Code.exe` 为目标的快捷方式，点击该入口在 Visual Studio Code 中打开规范 root，且不显示伴随的命令提示符窗口

#### Scenario: Windows 默认 Code 缺少有效 GUI 入口
- **WHEN** Windows 用户未配置 `pin.ide`，系统能发现 `code.cmd` 但无法定位或验证其对应的 Visual Studio Code GUI executable
- **THEN** pin 明确失败并报告默认 IDE 解析错误，且不创建以批处理 shim 为目标的快捷方式

#### Scenario: 既有批处理目标不被重复 pin 暗中更新
- **WHEN** 某 repository 的既有受管快捷方式已指向 `code.cmd` 且用户再次 pin 同一规范 root
- **THEN** 命令遵循既有幂等规则保持快捷方式不变，用户需先 unpin 再 pin 才能获得 v1.2.1 的 GUI 目标

### Requirement: Linux 启动器行为
在 Linux 上，系统 SHALL 将受管启动器保存为 `${XDG_DATA_HOME:-$HOME/.local/share}/applications/git-pin-<repo-name>.desktop`。Desktop Entry SHALL 至少声明 `Type=Application`、repository 显示名称、可用的 IDE 图标、`Terminal=false`，以及安全编码的已固化 IDE executable 和规范 root。成功 pin 后文件 SHALL 具备桌面环境启动所需权限，并在可用时刷新 desktop entry 数据库；刷新工具缺失不得导致 pin 失败。

#### Scenario: 创建 Linux desktop entry
- **WHEN** 用户在 Linux 上以有效 `pin.ide` 成功 pin 名为 `foo` 的 repository 且未设置 `XDG_DATA_HOME`
- **THEN** `~/.local/share/applications/git-pin-foo.desktop` 是有效、可启动的 Desktop Entry，并在所选 IDE 中打开规范 root

#### Scenario: 遵循 XDG_DATA_HOME
- **WHEN** 用户设置了绝对路径形式的 `XDG_DATA_HOME` 后 pin repository
- **THEN** 系统在该目录的 `applications` 子目录创建启动器，而不写入默认目录

#### Scenario: Linux 路径安全编码
- **WHEN** IDE executable 或 repository root 包含空格、Desktop Entry 字段特殊字符或非 ASCII 字符
- **THEN** 启动器按 Desktop Entry 规范编码 executable 与 root，并且启动时不发生参数拆分或命令注入

### Requirement: macOS 启动器行为
在 macOS 上，系统 SHALL 将每个受管启动器保存为 `$HOME/Applications/Git Pin/<repo-name>.app` 标准应用 bundle。Bundle SHALL 具有稳定且不会与其他 repository 冲突的标识、可执行启动入口和描述性元数据，并 SHALL 使用 pin 时解析并固化的 IDE executable 以规范 root 作为单个位置参数启动，使入口可从 Finder 打开并可被 Spotlight 索引。系统不得修改系统级 `/Applications`，也不得要求管理员权限。

#### Scenario: 创建 macOS app bundle
- **WHEN** 用户在 macOS 上以有效 `pin.ide` 成功 pin 名为 `foo` 的 repository
- **THEN** `$HOME/Applications/Git Pin/foo.app` 是可由 Finder 启动的有效应用 bundle，启动后所选 IDE 打开规范 root

#### Scenario: macOS 路径安全传递
- **WHEN** IDE executable 或 repository root 包含空格、shell 元字符或非 ASCII 字符
- **THEN** bundle 启动入口将 executable 与整个 root 分别作为程序和单个数据参数处理，不将任一值解释为 shell 内容

#### Scenario: macOS 用户级安装
- **WHEN** 非管理员用户 pin repository
- **THEN** 系统仅写入该用户的 Applications 目录并成功创建入口，不请求提权

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
