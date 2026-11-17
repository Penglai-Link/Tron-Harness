---
name: tron-theme-spec
description: 制作、修改和打包 Tron 的 skin.json 或 ZIP 主题包，覆盖 Tron 控件、共享 Agent UI 的 Chat、Composer、A2UI 与动效、主题装饰及加载边界。用户制作主题、调整组件外观或排查导入加载时使用。
metadata:
  version: 1.3.1
---

# Tron 主题规范与制作

交付可直接从「设置 → 外观」导入或从在线主题市场安装的 JSON / ZIP 主题包。按用户指定的风格、模式和资源制作；每个主题必须同时提供浅色和深色配色。不要靠修改 Tron 源码来制作普通主题包。

脚本需要 Python 3.9+，仅使用标准库。下文的 `scripts/` 路径相对此 SKILL.md 所在目录；执行时解析为该目录下的绝对路径，输出目录使用用户项目路径。

## 范围与 Agent UI 主题技能

本技能只负责 Tron 可导入的主题包，不维护 `@personai/agent-ui-*` 源码，也不替代 Framework 中 `eliteforge-agent-ui-theme` 的完整主题契约。[agent-ui.md](references/agent-ui.md) 是 Tron 消费侧的裁剪说明：只列主题作者能使用的公开变量、挂点和动效边界。Chat、Composer、A2UI、Motion、Terminal 的源码入口、默认值、React / Vue 一致性、包导出和发布维护仍以 `eliteforge-agent-ui-theme` 与目标版 Agent UI 源码为准；两者冲突时以实际源码和包导出为准。

## Output Path

1. 用户明确指定的主题源目录优先。
2. 未指定时，在当前用户项目的 `themes/<主题ID>/` 创建源目录；不写入能力包安装缓存。
3. 主文件为源目录中的 `skin.json`，资源放在其内。ZIP 默认生成于源目录同级 `<主题ID>.zip`，明确指定输出文件时按指定路径。创建父目录，拒绝覆盖已有产物；后续修改以源目录为准。

## 工作流程

1. 阅读 [format-and-loading.md](references/format-and-loading.md)，确认协议版本、包格式、资源和加载限制。
2. 用 `scripts/theme_pack.py init <目录> --id <小写唯一ID> --name <名称>` 建立包含 light / dark 两套配色的主题。脚本以 skill 目录为基准找到模板，不依赖工作目录。内置 ID `default`、`brand`、`matrix` 以及退役 ID `light`、`matrix-dark`、`matrix-particles` 不能用于新主题。
3. 制作边框、藤蔓、花瓣或独立动画时，阅读 [effect-anchors.md](references/effect-anchors.md)，使用组件的 `data-theme-anchor` 装饰层；`data-theme-part` 只用于普通 CSS 视觉定位，不会自动创建装饰层。不要给业务组件设置定位或尺寸。涉及聊天、输入区、A2UI、问答、图表或动效时读取 [agent-ui.md](references/agent-ui.md)，并结合 [tokens.md](references/tokens.md) 选择当前公开变量。先搭配全局表面色、正文色、强调色，再调整按钮、输入框、数据行。
4. 能力和扩展配置页可用的稳定挂点为 `capability-packs-page`、`capability-center-card`、`capability-pack-detail-page`、`capability-configuration-page`、`capability-extension-tabs`，局部文字色变量为 `--tron-capability-effective-fg`、`--tron-capability-inactive-fg`、`--tron-capability-provider-fg`。旧页面的 `capability-pack-card`、`capability-implementation`、`capability-provider` 等挂点已不再由当前页面声明，不能继续作为有效接口。
5. 图片和字体放在包内的 `assets/`。优先制作离线 ZIP；仅用配色时也可交付单个 `skin.json`。远程背景和字体仅支持 HTTPS，远程字体需要跨域许可；CSS 只能放在包内，不能引用远程文件。不要将用户提供的本地路径原样写入清单。
6. 运行 `python3 scripts/theme_pack.py validate <目录或ZIP或JSON>`。修正报错；未知变量警告须核实，不能把未消费的变量当成已实现效果。脚本不渲染 CSS，还需检查可读性、按钮各状态、行选中状态、聊天 Markdown、Composer、QuestionCard 和图表。Tron 与共享 Agent UI 使用不同的控件变量前缀，不能假设 `--tron-*` 自动影响 A2UI。
7. 运行 `python3 scripts/theme_pack.py pack <目录> <输出ZIP>`。脚本只打包清单及引用资源，确保 ZIP 根目录直接有 `skin.json`，不会混入系统文件或源目录外层。
8. 有可操作的 Tron 实例时，按用户授权导入或在「外观 → 在线主题」安装并选择，在实际业务页面检查重复卡片/聊天消息的独立装饰、滚动和缩放、弹窗、系统减少动态效果，并检查按钮 hover/active/focus/disabled、输入框、斑马纹/hover/selected 数据行、Composer、聊天历史索引、图片缩略图和弹窗。切回内置主题检查样式清理，刷新检查恢复。没有实例时，明确报告仅完成静态校验，不声称验证了 UI。
9. 交付包文件、关键配色/组件变量、导入步骤及验证边界。`version` 是包协议版本，固定为 3，不是主题发布版本；更新主题内容时换用新的 `id`，并同时提供两套配色。

## 随附示例

[Ocean 组件级示例](assets/ocean-detail/skin.json) 是可直接导入的完整配色示例，演示按钮状态、输入框聚焦、表格行样式和少量共享 Agent UI 变量。需要改造现有主题时可复制此目录并更换 `id`；从基础配色开始则使用 `assets/light/` 或 `assets/dark/` 模板。

示例源文件在 `assets/` 维护，ZIP 按需生成，不维护第二份二进制副本。校验和打包命令见 [加载说明](references/format-and-loading.md#随附-ocean-示例)。

## 共享 Agent UI 与动效

- Chat 使用 `--agent-ui-chat-*-color` 外部覆盖入口；Markdown 与语法高亮使用 `--agent-ui-markdown-*` / `--agent-ui-syntax-*`。旧 `--tron-markdown-*` / `--tron-syntax-*` 不能作为当前共享聊天的有效接口。
- Composer 使用 `--agent-ui-composer-*` 及共享控件变量，挂点为 `composer` 与输入框的 `chat-input`；`--agent-ui-composer-skill-font-size` 和 `--agent-ui-composer-skill-max-width` 已不再生效。聊天历史索引使用 `--agent-ui-chat-history-*`，图片缩略图使用 `--agent-ui-chat-image-thumbnail-*`。
- A2UI 使用 `--agent-ui-*` 通用控件和 `--agent-ui-a2ui-*` 专用变量。QuestionCard 的语义结构由客户端组合，主题调整外观，不修改问题内容、生命周期或交互协议。
- 图表系列、图例、SVG 线条和阈值状态需成组检查。不要依赖内部 class 或 DOM 层级，使用公开 `data-theme-part`。
- 主题变量不能覆盖宿主的动效策略。Tron 当前显式配置出现时长 180ms、位移 8px 和控件过渡 120ms，这些参数的根级 CSS 变量会被配置优先级遮蔽；`reduced` 偏好仍保留 90ms / 2px 的弱动效，`off` 才完全停用。只有宿主未配置的参数才继续读取效果专用变量和通用变量。关闭或减弱动画时必须保留完整内容及表单草稿。
- Tron 终端读取 `--tron-terminal-*` ANSI 调色板；共享 Agent UI 终端的 `--ai-terminal-*` 语义变量不由 Tron 消费，不能混用两套接口。

## 精细控制

- 按钮独立控制 `primary / secondary / outline / ghost / destructive / link` 的背景、前景、边框、悬停与按下状态。组件变量不需要改变全局强调色。
- 数据行独立控制普通、斑马纹、悬停、选中、焦点、行高、单元格内边距和分隔线。固定列跟随所在行的背景。
- 版本 3 可使用经过校验的包内 CSS，通过普通 CSS 选择器覆盖 Tron Web 页面内的匹配组件，并用 `data-theme-effect-layer="global"` 覆盖窗口，或用 `app-shell / sidebar / page / panel / card / chat / chat-message / dialog` 把装饰贴到各个组件实例；详见锚点参考；业务组件的尺寸、间距和布局属性受保护，只有标准特效选择器可使用受限定位和尺寸。CSS 不能引用包外资源，也不接受脚本、导入规则、嵌套规则或未列入白名单的 CSS 函数。不能用 JSON 匹配某条业务数据的 ID。
- `resources.background` 会同时用于工作台和设置页；用户在「外观」启用自定义背景效果或背景图时，宿主会隐藏皮肤背景。`--tron-sidebar-bg-opacity` 与 `--tron-sidebar-bg-blur` 属于本机侧栏偏好，由宿主覆盖，不要写进主题包冒充效果。
- 保留错误、警告、成功的可区分性；确保正文和禁用文本可读。终端与图谱颜色使用 HEX/RGB；代码、终端保持等宽字体。背景不强行铺到所有内容面板。
- 不自动安装到运行中后端的磁盘目录或改 `active.json`。正常交付走「外观」导入，或「在线主题」安装；文件模式的限制与恢复方式见加载参考。
