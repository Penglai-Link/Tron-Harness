---
name: tron-theme-authoring
description: 编写、修改和打包 Tron 的 skin.json 或 ZIP 主题包，支持按钮状态、输入框、表格数据行、背景和字体。用户要求制作 Tron 主题、调色、调整组件外观或排查主题导入加载时使用。
---

# Tron 主题包编写

交付可直接从「平台配置 → 外观 → 导入主题」加载的 JSON 或 ZIP。按用户指定的风格、模式和资源制作；没有指定时，从浅色模板开始。不要靠修改 Tron 源码来制作普通主题包。

脚本需要 Python 3.9+，仅使用标准库。下文的 `scripts/` 路径相对此 SKILL.md 所在目录；执行时解析为该目录下的绝对路径，输出目录使用用户项目路径。

## 工作流程

1. 阅读 [format-and-loading.md](references/format-and-loading.md)，确认包格式、资源和加载限制。
2. 用 `scripts/theme_pack.py init <目录> --id <小写唯一ID> --name <名称> --mode Light` 建立主题；深色用 `Dark`。脚本以 skill 目录为基准找到模板，不依赖工作目录。
3. 根据需要阅读 [tokens.md](references/tokens.md)。先搭配全局表面色、正文色、强调色，再调整按钮、输入框、数据行。`colors` 和 `layout` 都是 CSS 自定义属性映射，重复键以 `layout` 为准。
4. 图片和字体放在包内的 `assets/`。优先制作离线 ZIP；仅用配色时也可交付单个 `skin.json`。远程资源仅支持 HTTPS，远程字体需要跨域许可。不要将用户提供的本地路径原样写入清单。
5. 运行 `python3 scripts/theme_pack.py validate <目录或ZIP或JSON>`。修正报错；未知变量警告须核实，不能把未消费的变量当成已实现效果。脚本不渲染 CSS，还需检查可读性、按钮各状态及行选中状态。
6. 运行 `python3 scripts/theme_pack.py pack <目录> <输出ZIP>`。脚本只打包清单及引用资源，确保 ZIP 根目录直接有 `skin.json`，不会混入系统文件或源目录外层。
7. 有可操作的 Tron 实例时，按用户授权导入并选择，展开外观页「组件预览」，检查按钮 hover/active/focus/disabled、输入框、斑马纹/hover/selected 数据行和弹窗。切回内置主题检查样式清理，刷新检查恢复。没有实例时，明确报告仅完成静态校验，不声称验证了 UI。
8. 交付包文件、关键配色/组件变量、导入步骤及验证边界。更新版本用新的 `id`；`version` 是协议版本，仍固定为 1。

## 随附示例

[Ocean 组件级示例](assets/ocean-detail/skin.json) 是可直接导入的完整配色示例，演示按钮状态、输入框聚焦和表格行样式。需要改造现有主题时可复制此目录并更换 `id`；从基础模式开始则使用 `assets/light/` 或 `assets/dark/` 模板。

示例源文件在 `assets/` 维护，ZIP 按需生成，不维护第二份二进制副本。校验和打包命令见 [加载说明](references/format-and-loading.md#随附-ocean-示例)。

## 精细控制

- 按钮独立控制 `primary / secondary / outline / ghost / destructive / link` 的背景、前景、边框、悬停与按下状态。组件变量不需要改变全局强调色。
- 数据行独立控制普通、斑马纹、悬停、选中、焦点、行高、单元格内边距和分隔线。固定列跟随所在行的背景。
- 包只注入根变量，不接受 CSS 选择器、JS 或任意样式表；不能用 JSON 匹配某条业务数据的 ID。业务组件若要单独调整一行/一个按钮，可在该元素的 `style` 上覆盖相同变量；主题作者通过已公开的组件类别和状态控制外观。
- 保留错误、警告、成功的可区分性；确保正文和禁用文本可读。终端与图谱颜色使用 HEX/RGB；代码、终端保持等宽字体。背景不强行铺到所有内容面板。
- 不自动安装到运行中后端的磁盘目录或改 `active.json`。正常交付走导入界面；文件模式的限制与恢复方式见加载参考。
