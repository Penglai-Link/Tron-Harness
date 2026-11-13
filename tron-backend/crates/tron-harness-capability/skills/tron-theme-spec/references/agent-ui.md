# 共享聊天、Composer、A2UI 与动效

Tron 从 npm 安装 `@personai/agent-ui-*` 公共包；当前仓库依赖 `1.0.0-preview.12`，以 `tron-frontend/package.json` 的统一版本为准。主题包可以把颜色和控件变量写进 `skin.json` 的 `variants.light` / `variants.dark` / `layout`；需要局部外观时使用版本 3 的包内 CSS。公共包只渲染组件，皮肤安装、资源加载、CSS 作用域和清理由 Tron 管理。

本文件只描述 Tron 消费侧的公开接口。React / Vue 源码、默认值、包导出、完整变量目录和发布维护属于 `eliteforge-agent-ui-theme` 与 Agent UI 仓库；本文件不能替代那份契约。

## 变量归属和优先级

```text
Light / Dark 基底
  └─ skin.json variants → layout → 资源字体/背景
       ├─ --tron-*                  Tron Button / Input / Table / 页面
       ├─ --agent-ui-chat-*-color   共享聊天颜色入口
       ├─ --agent-ui-markdown-*     Markdown
       ├─ --agent-ui-syntax-*       代码高亮
       ├─ --agent-ui-composer-*     输入区
       └─ --agent-ui-* / --agent-ui-a2ui-*  通用控件与 A2UI

效果专用配置 → 宿主通用配置 → 效果专用 CSS 变量 → 通用 CSS 变量 → 默认值
```

`--color-*` 为语义回退，适合统一底色、正文、焦点、错误/警告/成功色。Tron 按钮变量不控制共享 A2UI 按钮，主题需分别检查。

Chat 根节点会生成 `--agent-ui-chat-surface`、`--agent-ui-chat-text` 等局部派生值。根级主题应设置 `--agent-ui-chat-surface-color`、`--agent-ui-chat-text-color`、`--agent-ui-chat-border-color`、`--agent-ui-chat-primary-color` 等外部入口，直接在祖先覆盖派生变量可能被局部声明遮蔽。

共享 Markdown 使用 `--agent-ui-markdown-*` 和 `--agent-ui-syntax-*`。旧 `--tron-markdown-*`、`--tron-syntax-*` 不再控制当前共享 Chat；`--fgColor-*` 等内部别名也不能作为稳定入口。

字体可用 `--agent-ui-font`、`--font-sans`；`resources.font` 会由 Tron 加载并设置宿主字体。`--tron-system-font-size` 和共享 `--agent-ui-system-font-size` 属于用户字号偏好，不用皮肤伪装成组件字号。独立统计字号可用专用变量。

## 公开挂点

| `data-theme-part` | 用途 |
| --- | --- |
| `chat` | 共享聊天整体 |
| `chat-message-user` / `chat-message-assistant` / `chat-message-system` | 用户 / 助手 / 系统通知消息 |
| `chat-history` | 左侧对话历史索引 |
| `chat-input` | 输入框 |
| `chat-image-thumbnail` | 用户附件和历史图片缩略图 |
| `chat-mermaid` | Mermaid 图容器 |
| `composer` | 独立或 Chat 内嵌输入区根 |
| `a2ui` | A2UI Surface 根 |
| `a2ui-<组件名小写>` | A2UI 组件，例如 `a2ui-questioncard`、`a2ui-chart` |
| `a2ui-question-intro/header/title/navigation/nav-button` | 问答介绍、标题、导航 |
| `a2ui-question-options/option/index/radio/content/arrow` | 问答选项区域 |
| `a2ui-question-actions/other/answer/confirm/skip` | 问答输入和操作 |

表格中的斜杠表示多个完整挂点，例如 `a2ui-question-title`。QuestionCard 选项的 `data-selected="true"` 表示选中，根节点 `data-status` 表示生命周期。不要把状态名或内部 CSS class 当新增业务协议。

`data-theme-part` 是视觉定位点，不自动创建特效层。`data-theme-effect-layer` 只支持 [装饰协议](effect-anchors.md) 列出的名称；A2UI 挂点不能直接当作任意新特效层名称。

共享 Chat / Composer 还会在宿主节点上声明 `data-theme-anchor="chat"`、`"chat-input"`、`"chat-message"`、`"composer"`、`"dialog"`，供 Tron 安装装饰层。锚点与 `data-theme-part` 同节点不等于可互相替代：特效选择器只认 `data-theme-effect-layer`，而装饰层只挂到 `data-theme-anchor`。

## Chat、Composer 与历史索引

Chat 的公开颜色入口使用 `-color` 后缀；组件内部会根据它派生直接令牌。常用项包括：

- `--agent-ui-chat-surface-color`、`--agent-ui-chat-surface-muted-color`、`--agent-ui-chat-surface-soft-color`、`--agent-ui-chat-surface-strong-color`
- `--agent-ui-chat-text-color`、`--agent-ui-chat-text-muted-color`
- `--agent-ui-chat-border-color`、`--agent-ui-chat-focus-color`、`--agent-ui-chat-hover-color`
- `--agent-ui-chat-primary-color`、`--agent-ui-chat-on-primary-color`、`--agent-ui-chat-danger-color`

消息操作行、压缩详情和系统通知沿用聊天正文、焦点和间距变量；普通消息和流式 Markdown 不新增逐字动画。不要在主题里通过 `display` 改变消息操作或历史索引的显示策略。

对话历史索引位于聊天左侧原有留白，主题不应通过增大正文或输入框 padding 容纳它。变量：

| 变量 | 默认值 / 用途 |
| --- | --- |
| `--agent-ui-chat-history-inset` | `2px`，距聊天左边缘 |
| `--agent-ui-chat-history-hit-width` | `16px`，透明点击区域宽度 |
| `--agent-ui-chat-history-step` | `12px`，轮次间距 / 按钮高度 |
| `--agent-ui-chat-history-dot-size` | `3px`，微点直径和竖线宽度 |
| `--agent-ui-chat-history-active-size` | `9px`，当前竖线高度 |
| `--agent-ui-chat-history-color` | 次要文字色混合 65% 不透明度 |
| `--agent-ui-chat-history-active-color` | 聊天正文色，可改为主题强调色 |
| `--agent-ui-chat-history-preview-width` | `260px`，同时受聊天容器宽度约束 |
| `--agent-ui-chat-history-preview-shadow` | `0 4px 16px rgb(0 0 0 / 10%)` |
| `--agent-ui-chat-history-preview-font-size` | `13px`，问题摘要字号 |
| `--agent-ui-chat-history-preview-detail-font-size` | `12px`，回复摘要字号 |

图片附件与结构化历史图片默认是紧凑缩略图，点击后在预览层查看原图。变量：

| 变量 | 默认值 |
| --- | --- |
| `--agent-ui-chat-image-thumbnail-width` | `112px` |
| `--agent-ui-chat-image-thumbnail-height` | `80px` |
| `--agent-ui-chat-image-thumbnail-radius` | `10px` |

批量或窄屏下缩略图会自动换行；用户消息右对齐，助手和系统消息不使用该分组。不要用 CSS 改写图片 URL、预览行为或把缩略图尺寸传给模型。

Composer 根挂点为 `data-theme-part="composer"` / `data-theme-anchor="composer"`，输入框保留 `chat-input`。独立输入区和 Chat 内嵌输入区共用 `packages/composer-core/src/styles.css`，主题要同时检查两处。尺寸、间距、字体优先复用 `--agent-ui-control-height`、`--agent-ui-control-padding`、`--agent-ui-control-radius`、`--agent-ui-control-font-size`、`--agent-ui-gap`、`--agent-ui-panel-padding`、`--agent-ui-panel-radius`；输入区专用项使用 `--agent-ui-composer-*`，例如：

- 布局与尺寸：`--agent-ui-composer-min-height`、`--agent-ui-composer-editor-min-height`、`--agent-ui-composer-padding`、`--agent-ui-composer-radius`、`--agent-ui-composer-row-gap`、`--agent-ui-composer-inline-gap`
- 控件与图标：`--agent-ui-composer-icon-size`、`--agent-ui-composer-icon-control-size`、`--agent-ui-composer-button-padding`、`--agent-ui-composer-focus-width`、`--agent-ui-composer-focus-offset`
- 附件与队列：`--agent-ui-composer-attachment-height`、`--agent-ui-composer-attachment-radius`、`--agent-ui-composer-file-width`、`--agent-ui-composer-file-padding`、`--agent-ui-composer-image-width`、`--agent-ui-composer-queue-max-height`
- 菜单与提示：`--agent-ui-composer-menu-max-height`、`--agent-ui-composer-menu-padding`、`--agent-ui-composer-menu-radius`、`--agent-ui-composer-suggestion-padding`
- 字体与状态：`--agent-ui-composer-font-size`、`--agent-ui-composer-meta-font-size`、`--agent-ui-composer-disabled-opacity`、`--agent-ui-composer-hover-brightness`

技能标签沿用输入框的字体与尺寸，以保持光标对齐；旧的 `--agent-ui-composer-skill-font-size`、`--agent-ui-composer-skill-max-width` 已不再生效。Composer 不在组件根部赋默认颜色，以避免遮蔽祖先继承；主题不要为“列全变量”而给根节点重复赋值。

## A2UI 与问答

优先复用 `--agent-ui-control-height`、`--agent-ui-control-padding`、`--agent-ui-control-radius`、`--agent-ui-control-font-size`、`--agent-ui-font`、`--agent-ui-gap`、`--agent-ui-panel-padding`、`--agent-ui-panel-radius` 等通用接口。A2UI 专用外观使用 `--agent-ui-a2ui-*`，基础色有 `--color-*` 回退。

QuestionCard 由客户端固定组合，原生问答和 Agent Catalog 共用。默认内边距为 `0.75rem`，选项标题、说明左对齐，底部输入可多行。主题只调外观，保持导航、其他答案、确认、跳过、禁用和历史完成态可用。

以下可放入 `skin.json` 对应映射，不需要 CSS 绕过布局保护：

```json
{
  "variants": {
    "light": {
      "--agent-ui-chat-surface-color": "#f8fbff",
      "--agent-ui-chat-text-color": "#15283f",
      "--agent-ui-a2ui-color-card-border": "#cbd8e8",
      "--agent-ui-a2ui-chart-color-1": "#245b8a",
      "--agent-ui-a2ui-chart-color-2": "#28744c"
    }
  },
  "layout": {
    "--agent-ui-control-radius": "8px",
    "--agent-ui-a2ui-questioncard-padding": "0.75rem",
    "--agent-ui-a2ui-statistic-value-font-size": "2rem"
  }
}
```

A2UI 表单响应区新增 `--agent-ui-a2ui-form-background`、`--agent-ui-a2ui-form-padding`、`--agent-ui-a2ui-form-gap`、`--agent-ui-a2ui-form-max-width`、`--agent-ui-a2ui-form-radius` 和 `--agent-ui-a2ui-surface-max-width`；窄屏会按容器宽度收缩。不要用固定宽度破坏响应式布局。

## 图表和统计

图表、迷你趋势图及图例共用 `--agent-ui-a2ui-chart-color-1` 至 `-5`，阈值条的成功、警告、错误色也需可区分。SVG 外观包含 `--agent-ui-a2ui-chart-stroke-width`、`--agent-ui-a2ui-chart-point-radius`、`--agent-ui-a2ui-chart-label-font-size` 等。坐标属于数据，不通过主题修改。

检查真实 computed style 和 SVG stroke / fill，不能只确认 CSS 文件中有变量。统计值、标题、图例、折叠摘要在深浅色和窄屏都应可读。

## 动效边界

Tron 使用 `TronMotionProvider` 和宿主维护的 React Bits 适配器。用户偏好有 `system`、`reduced`、`off`；皮肤不改变偏好，也不附带 JavaScript 动画库。

共享参数包括 `--agent-ui-motion-duration/delay/distance/intensity/easing`，`enter-*`、`fade-*` 专用参数，以及 `transition-duration`、`spin-duration`、`pulse-duration`、`pulse-easing`。完整拼写见 `tokens.json`。

当前 Tron 显式配置 `duration=180`、`distance=8`、`transitionDuration=120`，这些配置优先于效果专用和通用 CSS 变量；只有宿主未显式配置的参数才读取效果专用变量，再读取通用变量。主题不能承诺只改通用时长变量就改变当前 Tron 的出现动画。

`reduced` 偏好不是完全静止：Tron 的适配器仍以 90ms、2px、强度 0.35 播放一次弱出现效果；`off` 才把效果配置为关闭。Portal 内的共享控件通过 `data-tron-motion-policy` 继承同一策略。主题不能覆盖该策略，也不能通过重建表单重播动画；关闭或减弱时必须保留完整内容、草稿和可操作性。

QuestionCard / ApprovalCard 出现、问题切换、Tabs、Accordion / Collapsible 使用共享策略；普通历史消息和流式 Markdown 没有新增逐字动画。特效层还要独立遵循系统 reduced-motion；不能把包内 CSS 装饰、共享 Motion 和 Portal 动效当成同一个开关覆盖承诺。

## 终端

Tron 直接使用 `@personai/agent-ui-terminal` 的渲染器，但只把 ANSI 调色板接到 `--tron-terminal-*`，再通过 `themeTokens.ansi` 传给 xterm；它不消费共享包的 `--ai-terminal-*` 语义变量。制作 Tron 主题时只改 `--tron-terminal-*`；维护 Agent UI 终端主题时才使用 `--ai-terminal-*`、`TerminalThemeTokenOverrides`、`setThemeTokens()` 和 `refreshTheme()`。

## 验证和维护

静态检查运行 `theme_pack.py validate`，实际 UI 检查主聊天、侧边聊天、独立窗口、QuestionCard 未答/已答、审批、图表/图例、统计、Composer 独立与内嵌、历史索引、图片缩略图、输入焦点及窄屏。动态切换时确认草稿保留、历史完成卡片不重播、减弱/关闭后仍可操作，切回内置主题清理旧覆盖。

更新本参考和 `tokens.json` 时核对 Agent UI 源码的 `packages/chat-{react,vue}/src/styles.css`、`packages/chat-core/src/history-rail.css`、`packages/chat-theme/src/`、`packages/composer-core/src/styles.css`、`packages/a2ui-core/`、`packages/motion-core/`、`packages/terminal/`，以及 `docs/a2ui-theme.md`、`docs/a2ui-theme-tokens.json`、`docs/composer-theme.md`、`docs/composer-theme-tokens.json`、`docs/motion.md`、`docs/motion-tokens.json`、`docs/integration-guide.md`；再核对 Tron 实际依赖和宿主配置。上述路径相对 Agent UI 仓库，不是要求在技能安装目录中查找。
