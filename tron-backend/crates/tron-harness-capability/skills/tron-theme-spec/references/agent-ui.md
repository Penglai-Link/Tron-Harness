# 共享聊天、A2UI 与动效

Tron 从 npm 安装 `@personai/agent-ui-*` 公共包。颜色与控件变量可以进入 `skin.json` 的 `colors` / `layout`；需要局部外观时使用版本 2 的包内 CSS。公共包只渲染组件，皮肤安装、资源加载、CSS 作用域和清理由 Tron 管理。

## 变量归属和优先级

```text
Light / Dark 基底
  └─ skin.json colors → layout → 资源字体/背景
       ├─ --tron-*                  Tron Button / Input / Table
       ├─ --agent-ui-chat-*-color   共享聊天颜色入口
       ├─ --agent-ui-markdown-*     Markdown
       ├─ --agent-ui-syntax-*       代码高亮
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
| `chat-message-user` / `chat-message-assistant` | 用户 / 助手消息 |
| `chat-input` | 输入框 |
| `a2ui` | A2UI Surface 根 |
| `a2ui-<组件名小写>` | A2UI 组件，例如 `a2ui-questioncard`、`a2ui-chart` |
| `a2ui-question-intro/header/title/navigation/nav-button` | 问答介绍、标题、导航 |
| `a2ui-question-options/option/index/radio/content/arrow` | 问答选项区域 |
| `a2ui-question-actions/other/answer/confirm/skip` | 问答输入和操作 |

表格中的斜杠表示多个完整挂点，例如 `a2ui-question-title`。QuestionCard 选项的 `data-selected="true"` 表示选中，根节点 `data-status` 表示生命周期。不要把状态名或内部 CSS class 当新增业务协议。

`data-theme-part` 是视觉定位点，不自动创建特效层。`data-theme-effect-layer` 只支持 [装饰协议](effect-anchors.md) 列出的名称；A2UI 挂点不能直接当作任意新特效层名称。

## A2UI 与问答

优先复用 `--agent-ui-control-height`、`--agent-ui-control-padding`、`--agent-ui-control-radius`、`--agent-ui-control-font-size`、`--agent-ui-font`、`--agent-ui-gap`、`--agent-ui-panel-padding`、`--agent-ui-panel-radius` 等通用接口。A2UI 专用外观使用 `--agent-ui-a2ui-*`，基础色有 `--color-*` 回退。

QuestionCard 由客户端固定组合，原生问答和 Agent Catalog 共用。默认内边距为 `0.75rem`，选项标题、说明左对齐，底部输入可多行。主题只调外观，保持导航、其他答案、确认、跳过、禁用和历史完成态可用。

以下可放入 `skin.json` 对应映射，不需要 CSS 绕过布局保护：

```json
{
  "colors": {
    "--agent-ui-chat-surface-color": "#f8fbff",
    "--agent-ui-chat-text-color": "#15283f",
    "--agent-ui-a2ui-color-card-border": "#cbd8e8",
    "--agent-ui-a2ui-chart-color-1": "#245b8a",
    "--agent-ui-a2ui-chart-color-2": "#28744c"
  },
  "layout": {
    "--agent-ui-control-radius": "8px",
    "--agent-ui-a2ui-questioncard-padding": "0.75rem",
    "--agent-ui-a2ui-statistic-value-font-size": "2rem"
  }
}
```

## 图表和统计

图表、迷你趋势图及图例共用 `--agent-ui-a2ui-chart-color-1` 至 `-5`，阈值条的成功、警告、错误色也需可区分。SVG 外观包含 `--agent-ui-a2ui-chart-stroke-width`、`--agent-ui-a2ui-chart-point-radius`、`--agent-ui-a2ui-chart-label-font-size` 等。坐标属于数据，不通过主题修改。

检查真实 computed style 和 SVG stroke / fill，不能只确认 CSS 文件中有变量。统计值、标题、图例、折叠摘要在深浅色和窄屏都应可读。

## 动效边界

Tron 使用 `TronMotionProvider` 和宿主维护的 React Bits 适配器。用户偏好有 `system`、`reduced`、`off`；皮肤不改变偏好，也不附带 JavaScript 动画库。

共享参数包括 `--agent-ui-motion-duration/delay/distance/intensity/easing`，`enter-*`、`fade-*` 专用参数，以及 `transition-duration`、`spin-duration`、`pulse-duration`、`pulse-easing`。完整拼写见 `tokens.json`。

当前 Tron 显式配置 `duration=180`、`distance=8`、`transitionDuration=120`，这些配置优先于效果专用和通用 CSS 变量；只有宿主未显式配置的参数才读取效果专用变量，再读取通用变量。主题不能承诺只改通用时长变量就改变当前 Tron 的出现动画。宿主关闭或减弱时应保持内容可见、取消动画，不通过重建表单播放效果。

QuestionCard / ApprovalCard 出现、问题切换、Tabs、Accordion / Collapsible 使用共享策略；普通历史消息和流式 Markdown 没有新增逐字动画。特效层还要独立遵循系统 reduced-motion；不能把包内 CSS 装饰、共享 Motion 和 Portal 动效当成同一个开关覆盖承诺。

## 验证和维护

静态检查运行 `theme_pack.py validate`，实际 UI 检查主聊天、侧边聊天、独立窗口、QuestionCard 未答/已答、审批、图表/图例、统计、输入焦点及窄屏。动态切换时确认草稿保留、历史完成卡片不重播、减弱/关闭后仍可操作，切回内置主题清理旧覆盖。

更新本参考和 `tokens.json` 时核对共享 UI 的 `packages/chat-{react,vue}/src/styles.css`、`packages/chat-theme/src/`、`packages/a2ui-core/`、`packages/motion-core/`，及 `docs/a2ui-theme.md`、`docs/a2ui-theme-tokens.json`、`docs/motion.md`、`docs/motion-tokens.json`；再核对 Tron 实际依赖和宿主配置。上述路径相对共享 UI 仓库，不是要求在技能安装目录中查找。
