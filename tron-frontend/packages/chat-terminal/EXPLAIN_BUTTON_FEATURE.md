# 命令解释按钮功能

## 功能描述

当用户在终端中执行命令后，如果命令有输出内容，会在命令输出的末尾显示一个灯泡图标（💡）。点击这个图标会弹出解释对话框，显示命令和输出的解释。

## 实现细节

### 核心逻辑

1. **命令完成检测**
   - 监听 `shellIntegrationAddon` 的 `commandFinish` 事件
   - 当命令执行完成时，调用 `addExplainButton` 方法

2. **按钮显示条件**
   - 命令有可见输出（非空白）
   - `onExplain` 回调函数已配置
   - 命令块有有效的结束标记

3. **按钮位置**
   - 使用 xterm.js 的 decoration API 在命令输出末尾添加装饰
   - 按钮相对于命令输出行偏移 10px
   - 按钮大小根据终端行高自适应（最小 14px）

### 代码位置

**主要文件：**
- `packages/chat-terminal/src/terminal/addons/blocks.ts`
  - `addExplainButton()` - 创建解释按钮
  - `openExplainForBlock()` - 打开解释对话框

- `packages/chat-terminal/src/terminal/index.ts`
  - 在 `commandFinish` 事件处理中调用 `addExplainButton`

**UI 组件：**
- `packages/chat-terminal/src/terminal/ui/explainButton.ts` - 灯泡按钮组件
- `packages/chat-terminal/src/terminal/ui/explainDialog.ts` - 解释对话框组件

**样式文件：**
- `packages/chat-terminal/src/terminal/styles/explain.scss`

## 与 Block 边框的关系

- **Block 边框（灰色边框）**：已禁用，不再显示
- **解释按钮**：独立功能，仍然保留
- **Hover 操作栏**：仍然保留，hover 到命令区域时显示

这样既保持了终端界面的简洁，又保留了 AI 辅助功能。

## 交互流程

1. 用户执行命令（例如：`ls`）
2. 命令执行完成，输出显示在终端
3. 在输出末尾显示灯泡图标 💡
4. 用户点击灯泡图标
5. 弹出解释对话框，显示命令和输出的 AI 解释
6. 用户可以点击刷新重新生成解释，或点击关闭按钮关闭对话框

## 清理机制

- 当超过最大渲染块数量时，自动清理旧的按钮
- 按钮的 decoration 和 DOM 元素都会正确释放
- 终端 dispose 时清理所有按钮资源
