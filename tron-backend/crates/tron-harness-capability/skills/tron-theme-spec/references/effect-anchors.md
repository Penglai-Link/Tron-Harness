# 组件边缘装饰与动画

主题包使用 `version: 3` 和 `resources.css`。Tron 创建装饰层，作者只写 CSS 和包内素材；不注入 HTML、脚本或修改组件布局。

## 挂点与特效层

Tron 的业务组件只声明 `data-theme-anchor="名称"`；`[data-theme-effect-layer="名称"]` 是主题 CSS 选择器，用来命中该实例内由 Tron 生成的装饰层。两者不是同一个属性：

- `data-theme-part` 是普通 CSS 的视觉定位点，不会创建装饰层。
- `data-theme-anchor` 决定装饰层挂在哪里。
- `data-theme-effect-layer` 决定装饰层能被哪些 CSS 选择器命中。

| 特效层名称 | 覆盖范围 |
| --- | --- |
| global | 整个 Web 视口，固定在窗口边缘 |
| app-shell | 工作台、设置、账户选择等外壳 |
| sidebar | 工作台桌面/移动侧栏、设置导航 |
| page | 工作台或设置页主体的可视区域 |
| panel | 右侧/底部面板、表格容器、任务看板列、空间管理面板 |
| card | 公共卡片、指标卡、任务卡、Agent 卡、主题卡；每张独立 |
| chat | 共享聊天面板，主聊天、Side Chat、独立窗口复用 |
| chat-message | 每条用户/助手消息气泡；随流式内容增高 |
| dialog | Dialog/Sheet 弹层；随弹层一起出现和消失 |

这是有限的公共语义名，不支持主题自行注册任意名称或绑定业务数据 ID。自绘 Canvas 内部、外部网页和系统窗口不提供这些 DOM 锚点。锚点的边界是当前组件可视区域，装饰超出的部分会裁剪；不要依赖越界覆盖相邻卡片。

工作台/设置外壳、侧栏、页面主体、右侧/底部面板、SurfaceCard / MetricCard / Card、表格容器、Dialog/Sheet、手写确认/编辑弹窗、任务看板列与任务卡、Agent 卡、主题选择卡以及共享聊天面板和每条消息都可能提供锚点；手写新组件必须显式声明锚点，不能靠扫描圆角 CSS 猜卡片。外部 webview、Canvas 内部节点、PDF 像素和 OS 窗口不属于 DOM 锚点。

## 植物四角示例

把透明 PNG 放在 ZIP 根目录下的 `assets/vines-corners.png`。所有 `url()` 都相对 **ZIP 根目录**，与 CSS 所在目录无关；四角以外保持透明，避免遮住正文。

```css
[data-theme-effect-layer="card"]::before,
[data-theme-effect-layer="chat-message"]::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: url("assets/vines-corners.png") center / 100% 100% no-repeat;
  animation: leaves-breathe 6s ease-in-out infinite;
}

[data-theme-effect-layer="panel"]::after {
  content: "";
  position: absolute;
  inset: 0;
  box-shadow: inset 0 0 12px #36784c33;
}

@keyframes leaves-breathe {
  from { opacity: 0.65; }
  to { opacity: 1; }
}
```

每个实例各有一个真实层和 `::before` / `::after` 两个伪元素，可以设置不同动画、延迟与方向。窗口四周装饰换用 `global`。浏览器会自动给关键帧名加主题 ID，避免不同主题重名。

`[data-theme-effect-layer]` 不带值时会匹配所有已定义类型，应谨慎使用；不要用 `*`、后代、兄弟、`:is()` 或 `:has()` 扩大范围。

## 权限与生命周期

- 使用完整选择器 `[data-theme-effect-layer="名称"]`，可附加 `::before` 或 `::after`。逗号列表中的每项都必须是标准特效选择器。
- 不支持后代/兄弟选择器、`:is()`、`:has()` 或状态伪类来定位特效层；普通业务 CSS 仍使用原有视觉属性。
- 特效专用属性：`position`（只能 `absolute`）、`inset / inset-block / inset-inline / top / right / bottom / left`、`width / height / min-width / min-height / max-width / max-height`、`transform-origin`、`pointer-events`（只能 `none`）。不开放 `z-index`、排版、外边距或点击权限。
- 关键帧只允许视觉属性，用 `transform` 和 `opacity` 做动画，不用定位或尺寸关键帧。禁止 `!important`、外部/数据 URL、`@import`、脚本、HTML；图片和字体沿用包资源校验。
- 宿主布局、裁剪容器和层级归 Tron 管理。装饰视口使用 `position: fixed`（global）或 `absolute`（组件）、`overflow: clip`、`contain: strict`、`pointer-events: none`、`aria-hidden` 和 `inert`；静态定位的宿主会临时得到定位上下文，清理时恢复。
- 层始终不参与排版、不接收输入，读屏忽略；避免浓重素材遮住内容和焦点提示。
- 系统启用减少动态效果时，所有特效层及伪元素停用动画和过渡，保留静态装饰。
- 主题仅为 CSS 引用的类型创建装饰。页面新增/删除组件时同步挂载/清理；滚动、面板拖拽、窗口缩放时跟随。切换主题清除旧节点、监听和资源，退出预览恢复当前主题。
- ResizeObserver 跟随尺寸，捕获 scroll 保持滚动容器边框贴合，MutationObserver 合并处理动态挂载/卸载；不会逐帧轮询全部元素。

## 验证

先运行 `theme_pack.py validate`，再打 ZIP 并验证 ZIP。实际页面检查：两张以上不同尺寸卡片、流式消息增高、卡片所在列表滚动、面板本身滚动、面板缩放、移动侧栏、弹窗关闭、减少动态效果、预览取消和切回内置主题。确认按钮能点击、键盘焦点正常、没有新增横向滚动条。报告静态校验和真实渲染各自验证了什么。
