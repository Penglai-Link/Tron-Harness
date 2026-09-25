# 变量与组件

优先使用下表中的稳定公共变量。`tokens.json` 是打包脚本用于拼写检查的目录；未知变量只警告，不代表该变量一定无效。不要在 JSON 中填入未实现的效果。

## 继承与优先级

浅色/深色基底 → `colors` → `layout` → 资源引擎生成的背景/字体变量。组件变量优先于该组件的语义色回退。单独设置 `--tron-button-primary-bg` 只改变主按钮，不改正文、图谱或其他控件。

`--color-primary` 等语义颜色会桥接到 `--primary` 等控件别名，显式别名优先。模板不复制这组别名，避免改了主色却被旧别名盖住。`--color-success` 是业务成功色，`--success` 是 UI 库别名；同理 error→destructive、accent→info。

全局配色建议成对调整：surface/on-surface、primary/on-primary、accent-container/on-accent-container、error-container/on-error-container、success-container/on-success-container、warning-container/on-warning-container。背景和表面有多层，不能只改 `--color-background`。

## 按钮

`--tron-button-{variant}-{property}`：

- variant：`primary`（Button default）、`secondary`、`outline`、`ghost`、`destructive`、`link`。
- property：`bg` 背景、`fg` 文字、`border` 边框、`hover-bg` / `hover-fg` 悬停、`active-bg` / `active-fg` 按下及展开。
- 通用禁用态：`--tron-button-disabled-bg` / `-fg` / `-border` / `-opacity`。
- 焦点：`--tron-button-focus-ring`、`--tron-button-focus-width`。
- 尺寸：`--tron-button-height` 及 `-xs` / `-sm` / `-lg`，图标按钮使用同组尺寸。
- 形状/文字：`--tron-button-radius`、`--tron-button-padding-x`（默认大小）、`--tron-button-gap`（默认大小）、`--tron-button-border-width`、`--tron-button-font-size`（默认大小）、`--tron-button-font-weight`、`--tron-button-shadow`。小号按钮保留字号/间距比例。

```json
{
  "--tron-button-primary-bg": "#245b8a",
  "--tron-button-primary-fg": "#ffffff",
  "--tron-button-primary-hover-bg": "#1b476f",
  "--tron-button-primary-hover-fg": "#ffffff",
  "--tron-button-primary-active-bg": "#12324f",
  "--tron-button-primary-active-fg": "#ffffff",
  "--tron-button-radius": "10px"
}
```

## 数据行

作用在公共 Table/DataTable，不会误改卡片列表或用户消息。选中态优先于悬停，固定列使用所在行的当前背景。

| 变量 | 效果 |
| --- | --- |
| `--tron-table-row-bg` / `-fg` | 普通行背景/文字 |
| `--tron-table-row-stripe-bg` | 偶数行背景；未设置时不强制斑马纹 |
| `--tron-table-row-hover-bg` / `-hover-fg` | 悬停行背景/文字 |
| `--tron-table-row-selected-bg` / `-selected-fg` | 已选中/展开行背景/文字 |
| `--tron-table-row-focus-ring` | 键盘焦点边框 |
| `--tron-table-row-height` | 最小行高（内容可撑开） |
| `--tron-table-cell-padding-x` / `-y` | 单元格水平/垂直内边距 |
| `--tron-table-row-border` / `-border-width` | 分隔线颜色/粗细 |
| `--tron-table-font-size` | 行字号 |
| `--tron-table-header-bg` / `-fg` / `-height` / `-font-weight` | 表头外观 |

业务代码可以在某个 `TableRow` 或 `Button` 上用 `style={{ "--tron-table-row-bg": "..." }}` 覆盖变量。包本身只设置根变量，不带业务行选择器。若单元格内容主动指定状态色，行前景色不会压掉它。

## 表单、容器和弹层

- Input/Textarea：`--tron-input-bg` / `-fg` / `-border` / `-hover-border` / `-placeholder` / `-focus-border` / `-focus-ring` / `-focus-width` / `-disabled-bg` / `-disabled-border` / `-disabled-fg` / `-disabled-opacity` / `-invalid-border` / `-radius` / `-height`（单行）/ `-padding-x`。
- Select：`--tron-select-bg` / `-fg` / `-border` / `-radius`，未设置时继承 input 对应变量。
- SurfaceCard：`--tron-card-bg` / `-fg` / `-border` / `-radius` / `-padding` / `-shadow`。MetricCard 共享颜色/圆角，保持自己的密度。
- Dialog/Sheet：`--tron-dialog-bg` / `-fg` / `-radius`；遮罩 `--tron-overlay-bg`。手写业务弹窗仍沿用语义表面色和遮罩变量。
- Select/Popover/DropdownMenu 弹层：`--tron-popover-bg` / `-fg`；菜单聚焦/高亮 `--tron-menu-item-hover-bg` / `-fg`。
- `--radius` 调整 Tailwind 通用圆角比例；固定几何形状（头像圆形、进度条）不跟随卡片圆角。`--tron-control-radius` 是按钮/输入/下拉的回退圆角。

组件可通过已有 className/style 明确覆盖公共样式，所以这些变量不承诺控制所有手写原生 button 或业务专属布局。开发新页面时应使用公共组件。

## Markdown、代码、图谱、终端

- Markdown 表面、正文、边框、链接、状态色默认跟随全局语义色；局部覆盖使用 `--tron-markdown-*`（如 `--tron-markdown-fg-color-accent`）。原 GitHub `--fgColor-*` 在子元素声明，不能直接通过包覆盖。
- Markdown 语法颜色用 `--tron-syntax-*`，例如 keyword / string / comment / constant / entity；完整已消费键见下文目录。
- CodeMirror 编辑器通过全局 `--color-*` 取色。代码与终端保持等宽字体。
- 图谱：`--tron-graph-active-edge` / `-edge` / `-highlight` / `-selected` / `-label-bg` / `-label-fg` / `-label-border` / `-grid`。节点分类色使用 `--tron-graph-group-1-color` / `-border` / `-soft`，序号 1–12，按分类名稳定分组。图谱 Canvas 与 DOM 图例都刷新。
- 终端 ANSI：`--tron-terminal-black/red/green/yellow/blue/magenta/cyan/white`，以及 bright-black 等八个亮色。主题内使用 HEX 或 RGB 值，避免 Canvas/第三方颜色解析器差异。
- 字体：`--font-sans` 可指定已安装系统字体，`resources.font` 加载包内字体且优先级更高。标题、表单和 Portal 弹层共享字体；字号设置 `--tron-system-font-size` 不允许主题覆盖。

## 边界

主题覆盖 Tron 的 Web 页面，包括独立聊天窗口。版本 2 的包内 CSS 仅作用于当前 Tron 页面主题命名空间，支持 `data-theme-part` 标记、伪元素和关键帧动画；尺寸、间距和布局属性受保护。外部网页/webview 的内容、截图/图片/PDF 原文、原生 OS 窗口边框、用户图标和第三方固定装饰不属于主题覆盖面。

## 公共组件变量目录

- `--tron-button-border-width`
- `--tron-button-destructive-active-bg`
- `--tron-button-destructive-active-fg`
- `--tron-button-destructive-bg`
- `--tron-button-destructive-border`
- `--tron-button-destructive-fg`
- `--tron-button-destructive-hover-bg`
- `--tron-button-destructive-hover-fg`
- `--tron-button-disabled-bg`
- `--tron-button-disabled-border`
- `--tron-button-disabled-fg`
- `--tron-button-disabled-opacity`
- `--tron-button-focus-ring`
- `--tron-button-focus-width`
- `--tron-button-font-size`
- `--tron-button-font-weight`
- `--tron-button-gap`
- `--tron-button-ghost-active-bg`
- `--tron-button-ghost-active-fg`
- `--tron-button-ghost-bg`
- `--tron-button-ghost-border`
- `--tron-button-ghost-fg`
- `--tron-button-ghost-hover-bg`
- `--tron-button-ghost-hover-fg`
- `--tron-button-height`
- `--tron-button-height-lg`
- `--tron-button-height-sm`
- `--tron-button-height-xs`
- `--tron-button-link-active-bg`
- `--tron-button-link-active-fg`
- `--tron-button-link-bg`
- `--tron-button-link-border`
- `--tron-button-link-fg`
- `--tron-button-link-hover-bg`
- `--tron-button-link-hover-fg`
- `--tron-button-outline-active-bg`
- `--tron-button-outline-active-fg`
- `--tron-button-outline-bg`
- `--tron-button-outline-border`
- `--tron-button-outline-fg`
- `--tron-button-outline-hover-bg`
- `--tron-button-outline-hover-fg`
- `--tron-button-padding-x`
- `--tron-button-primary-active-bg`
- `--tron-button-primary-active-fg`
- `--tron-button-primary-bg`
- `--tron-button-primary-border`
- `--tron-button-primary-fg`
- `--tron-button-primary-hover-bg`
- `--tron-button-primary-hover-fg`
- `--tron-button-radius`
- `--tron-button-secondary-active-bg`
- `--tron-button-secondary-active-fg`
- `--tron-button-secondary-bg`
- `--tron-button-secondary-border`
- `--tron-button-secondary-fg`
- `--tron-button-secondary-hover-bg`
- `--tron-button-secondary-hover-fg`
- `--tron-button-shadow`
- `--tron-card-bg`
- `--tron-card-border`
- `--tron-card-fg`
- `--tron-card-padding`
- `--tron-card-radius`
- `--tron-card-shadow`
- `--tron-dialog-fg`
- `--tron-dialog-radius`
- `--tron-input-bg`
- `--tron-input-border`
- `--tron-input-disabled-border`
- `--tron-input-disabled-bg`
- `--tron-input-disabled-fg`
- `--tron-input-disabled-opacity`
- `--tron-input-fg`
- `--tron-input-focus-border`
- `--tron-input-focus-ring`
- `--tron-input-focus-width`
- `--tron-input-height`
- `--tron-input-hover-border`
- `--tron-input-invalid-border`
- `--tron-input-padding-x`
- `--tron-input-placeholder`
- `--tron-input-radius`
- `--tron-menu-item-hover-bg`
- `--tron-menu-item-hover-fg`
- `--tron-overlay-bg`
- `--tron-popover-bg`
- `--tron-popover-fg`
- `--tron-select-bg`
- `--tron-select-border`
- `--tron-select-fg`
- `--tron-select-radius`
- `--tron-table-cell-padding-x`
- `--tron-table-cell-padding-y`
- `--tron-table-font-size`
- `--tron-table-header-bg`
- `--tron-table-header-fg`
- `--tron-table-header-font-weight`
- `--tron-table-header-height`
- `--tron-table-row-bg`
- `--tron-table-row-border`
- `--tron-table-row-border-width`
- `--tron-table-row-fg`
- `--tron-table-row-focus-ring`
- `--tron-table-row-height`
- `--tron-table-row-hover-bg`
- `--tron-table-row-hover-fg`
- `--tron-table-row-selected-bg`
- `--tron-table-row-selected-fg`
- `--tron-table-row-stripe-bg`

## Markdown 专属变量目录

- `--tron-markdown-bg-color-attention-muted`
- `--tron-markdown-bg-color-default`
- `--tron-markdown-bg-color-muted`
- `--tron-markdown-bg-color-neutral-muted`
- `--tron-markdown-border-color-accent-emphasis`
- `--tron-markdown-border-color-attention-emphasis`
- `--tron-markdown-border-color-danger-emphasis`
- `--tron-markdown-border-color-default`
- `--tron-markdown-border-color-done-emphasis`
- `--tron-markdown-border-color-muted`
- `--tron-markdown-border-color-neutral-muted`
- `--tron-markdown-border-color-success-emphasis`
- `--tron-markdown-fg-color-accent`
- `--tron-markdown-fg-color-attention`
- `--tron-markdown-fg-color-danger`
- `--tron-markdown-fg-color-default`
- `--tron-markdown-fg-color-done`
- `--tron-markdown-fg-color-muted`
- `--tron-markdown-fg-color-success`
- `--tron-markdown-focus-outline-color`
- `--tron-syntax-brackethighlighter-angle`
- `--tron-syntax-brackethighlighter-unmatched`
- `--tron-syntax-carriage-return-bg`
- `--tron-syntax-carriage-return-text`
- `--tron-syntax-comment`
- `--tron-syntax-constant`
- `--tron-syntax-constant-other-reference-link`
- `--tron-syntax-entity`
- `--tron-syntax-entity-tag`
- `--tron-syntax-invalid-illegal-bg`
- `--tron-syntax-invalid-illegal-text`
- `--tron-syntax-keyword`
- `--tron-syntax-markup-bold`
- `--tron-syntax-markup-changed-bg`
- `--tron-syntax-markup-changed-text`
- `--tron-syntax-markup-deleted-bg`
- `--tron-syntax-markup-deleted-text`
- `--tron-syntax-markup-heading`
- `--tron-syntax-markup-ignored-bg`
- `--tron-syntax-markup-ignored-text`
- `--tron-syntax-markup-inserted-bg`
- `--tron-syntax-markup-inserted-text`
- `--tron-syntax-markup-italic`
- `--tron-syntax-markup-list`
- `--tron-syntax-meta-diff-range`
- `--tron-syntax-storage-modifier-import`
- `--tron-syntax-string`
- `--tron-syntax-string-regexp`
- `--tron-syntax-sublimelinter-gutter-mark`
- `--tron-syntax-variable`
