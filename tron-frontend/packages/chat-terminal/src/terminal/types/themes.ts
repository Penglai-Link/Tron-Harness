import type { ITheme } from "@xterm/xterm";

/** 终端组件使用的语义颜色；字符串必须是浏览器可解析的 CSS 颜色值。 */
export interface TerminalThemeTokens {
  /** 终端画布的背景色。 */
  background: string;
  /** 终端正文和默认输出的前景色。 */
  foreground: string;
  /** 终端光标的颜色。 */
  cursor: string;
  /** 终端选中文本区域的背景色。 */
  selectionBackground: string;
  /** 模型会话卡片、命令说明弹窗等扩展面板的背景色。 */
  panelBackground: string;
  /** 扩展面板的主要前景色。 */
  panelForeground: string;
  /** 次要文本和辅助信息的颜色。 */
  muted: string;
  /** 弱化文本和低强调信息的颜色。 */
  subtle: string;
  /** 分隔线、描边和引用标记的颜色。 */
  border: string;
  /** 链接、激活状态和重点操作的强调色。 */
  accent: string;
  /** 行内代码、代码块和命令内容的背景色。 */
  codeBackground: string;
  /** 输入框和可编辑区域的背景色。 */
  inputBackground: string;
  /** 可交互元素悬停时的背景色。 */
  hoverBackground: string;
  /** 错误信息和失败状态的语义色。 */
  error: string;
  /** 警告信息和警告状态的语义色。 */
  warning: string;
  /** 成功信息和完成状态的语义色。 */
  success: string;
  /** 默认图标和图标描边的颜色。 */
  icon: string;
  /** xterm 原生主题覆盖项，包括 ANSI 调色板及其他终端颜色。 */
  ansi?: Partial<ITheme>;
}

export type TerminalThemeTokenOverrides = Partial<TerminalThemeTokens>;
