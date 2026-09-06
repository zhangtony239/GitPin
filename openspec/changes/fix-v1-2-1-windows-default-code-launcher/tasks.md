## 1. 配置选择边界

- [ ] 1.1 将 IDE 配置读取结果扩展为可区分“未配置时的默认 `code`”与显式 `pin.ide` 值的类型，并更新读取优先级、缺失值和错误测试
- [ ] 1.2 调整 Pin 应用编排，使 Windows 默认选择进入专用 GUI 解析，而显式配置及非 Windows 平台继续走现有原子 executable 解析

## 2. Windows 默认 VS Code 解析

- [ ] 2.1 实现 Windows 默认 `code` 解析：从 PATH 选中的 `code.cmd` 标准布局推导并验证相邻 `Code.exe`，同时支持 PATH 直接提供有效 GUI executable
- [ ] 2.2 对脚本 shim 缺少或对应 GUI executable 无效的情况返回带 shim/候选上下文的诊断，且禁止回退为 `.cmd`、`.bat` 或 `cmd.exe /c`
- [ ] 2.3 增加 Windows 解析 fixture 测试，覆盖标准安装布局、缺失 GUI、直接 `.exe`、路径空格/非 ASCII 及显式 `pin.ide=code` 不被默认适配重写

## 3. Windows launcher 回归覆盖

- [ ] 3.1 扩展 Windows 原生 Shell Link 测试，使用 `bin\code.cmd` 加 `Code.exe` 布局完成 pin，并断言读回 TargetPath 为 GUI `.exe`、图标来自 GUI executable、root 参数与工作目录准确
- [ ] 3.2 增加既有 `code.cmd` 受管 launcher 的重复 pin、inspect、list 和 unpin 兼容测试，确认 v1.2.1 不隐式迁移但仍可安全管理旧入口
- [ ] 3.3 验证默认 GUI 候选解析失败时不提交最终或临时 launcher，并保持现有冲突与原子创建语义

## 4. v1.2.1 发布收尾

- [ ] 4.1 将 crate/package 版本与 lockfile 更新为 `1.2.1`，检查发布 workflow 继续从唯一版本源派生 tag 和产物名称
- [ ] 4.2 更新中英文 README，记录 Windows 默认 Code 不再弹出命令窗口，并说明受影响的 v1.2 入口需执行 unpin 后重新 pin
- [ ] 4.3 运行格式化、测试、clippy 与 OpenSpec 严格校验，并在 Windows 上手工 smoke test 开始菜单入口直接打开 repository 且无命令提示符窗口
