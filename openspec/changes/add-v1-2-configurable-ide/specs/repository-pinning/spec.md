## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Windows 启动器行为
在 Windows 上，系统 SHALL 将受管启动器保存为 `%APPDATA%\Microsoft\Windows\Start Menu\Programs\.pinned_repo\<repo-name>.lnk`。快捷方式 SHALL 直接启动 pin 时解析并固化的 IDE executable，以规范 root 作为单个参数和工作目录，并使用该 executable 可用的图标。路径比较 SHALL 遵循 Windows 的不区分大小写语义。

#### Scenario: 创建 Windows 快捷方式
- **WHEN** 用户在 Windows 上以有效 `pin.ide` 成功 pin 名为 `foo` 的 repository
- **THEN** `.pinned_repo\foo.lnk` 存在，且其目标、参数和工作目录可使开始菜单入口在所选 IDE 中打开规范 root

#### Scenario: Windows 路径包含空格或非 ASCII 字符
- **WHEN** IDE executable 或 repository root 包含空格、引号可表示字符或非 ASCII 字符
- **THEN** 快捷方式保留准确路径，启动时将整个 root 作为一个参数传递且不执行路径中的内容

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
