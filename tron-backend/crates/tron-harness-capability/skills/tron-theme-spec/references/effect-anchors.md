# 组件边缘装饰与动画

主题包使用 `version: 2` 和 `resources.css`。Tron 创建装饰层，作者只写 CSS 和包内素材；不注入 HTML、脚本或修改组件布局。

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

## 权限与生命周期

- 使用完整选择器 `[data-theme-effect-layer="名称"]`，可附加 `::before` 或 `::after`。逗号列表中的每项都必须是标准特效选择器。`[data-theme-effect-layer]` 匹配所有已定义类型，应谨慎使用，避免重复叠加。
- 不支持后代/兄弟选择器、`:is()`、`:has()` 或状态伪类来定位特效层；普通业务 CSS 仍使用原有视觉属性。
- 特效专用属性：`position`（只能 `absolute`）、`inset / inset-block / inset-inline / top / right / bottom / left`、`width / height / min-width / min-height / max-width / max-height`、`transform-origin`、`pointer-events`（只能 `none`）。不开放 `z-index`、排版、外边距或点击权限。
- 关键帧只允许视觉属性，用 `transform` 和 `opacity` 做动画，不用定位或尺寸关键帧。禁止 `!important`、外部/数据 URL、`@import`、脚本、HTML；图片和字体沿用包资源校验。
- 宿主布局、裁剪容器和层级归 Tron 管理。层始终不参与排版、不接收输入，读屏忽略；避免浓重素材遮住内容和焦点提示。
- 系统启用减少动态效果时，所有特效层及伪元素停用动画和过渡，保留静态装饰。
- 主题仅为 CSS 引用的类型创建装饰。页面新增/删除组件时同步挂载/清理；滚动、面板拖拽、窗口缩放时跟随。切换主题清除旧节点、监听和资源，退出预览恢复当前主题。

## 验证

先运行 `theme_pack.py validate`，再打 ZIP 并验证 ZIP。实际页面检查：两张以上不同尺寸卡片、流式消息增高、卡片所在列表滚动、面板本身滚动、面板缩放、移动侧栏、弹窗关闭、减少动态效果、预览取消和切回内置主题。确认按钮能点击、键盘焦点正常、没有新增横向滚动条。报告静态校验和真实渲染各自验证了什么。
