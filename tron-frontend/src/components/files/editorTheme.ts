import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

/**
 * CodeMirror 主题，将每个可见颜色都映射到项目的 `--color-*` 令牌。由于项目
 * 仅通过切换 CSS 变量来切换 light / matrix-dark，编辑器无需任何 JS 即可跟随
 * 当前主题。
 */
export const tronEditorTheme = EditorView.theme(
  {
    "&": {
      height: "100%",
      backgroundColor: "transparent",
      color: "var(--color-on-surface)",
    },
    ".cm-scroller": {
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      lineHeight: "1.5",
      overflow: "auto",
    },
    ".cm-content": {
      caretColor: "var(--color-on-surface)",
      padding: "8px 0",
    },
    ".cm-line": {
      padding: "0 12px",
    },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--color-on-surface-variant)",
      border: "none",
      paddingLeft: "4px",
    },
    ".cm-activeLine": {
      backgroundColor: "var(--color-hover-overlay)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "transparent",
      color: "var(--color-on-surface)",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
      backgroundColor: "var(--color-accent-container)",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--color-on-surface)",
    },
    ".cm-matchingBracket": {
      backgroundColor: "var(--color-accent-container)",
    },
    ".cm-nonmatchingBracket": {
      backgroundColor: "var(--color-error-container)",
    },
    ".cm-foldGutter .cm-gutterElement": {
      color: "var(--color-on-surface-variant)",
    },
  },
  { dark: false },
);

const tronHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: "var(--color-accent)" },
  { tag: tags.comment, color: "var(--color-on-surface-variant)", fontStyle: "italic" },
  { tag: tags.string, color: "var(--color-success)" },
  { tag: tags.number, color: "var(--color-warning)" },
  { tag: tags.typeName, color: "var(--color-primary)" },
  { tag: tags.className, color: "var(--color-primary)" },
  { tag: tags.function(tags.variableName), color: "var(--color-accent)" },
  { tag: tags.propertyName, color: "var(--color-on-surface)" },
  { tag: tags.operator, color: "var(--color-on-surface-variant)" },
  { tag: tags.bool, color: "var(--color-warning)" },
  { tag: tags.null, color: "var(--color-warning)" },
  { tag: tags.regexp, color: "var(--color-success)" },
  { tag: tags.definition(tags.variableName), color: "var(--color-primary)" },
  { tag: tags.labelName, color: "var(--color-accent)" },
]);

export const tronSyntaxHighlighting = syntaxHighlighting(tronHighlightStyle);
