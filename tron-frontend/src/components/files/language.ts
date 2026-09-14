import type { LanguageSupport } from "@codemirror/language";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";

import { extensionOf } from "@/shared/fs/binaryGuard";

/**
 * 将文件名映射到 CodeMirror 语言扩展。没有匹配到具体语言时返回 null，
 * 编辑器会回退为纯文本，并使用通用的高亮/括号匹配扩展。
 */
export function resolveLanguage(fileName: string): LanguageSupport | null {
  switch (extensionOf(fileName)) {
    case "js":
    case "mjs":
    case "cjs":
      return javascript({ typescript: false, jsx: false });
    case "jsx":
      return javascript({ typescript: false, jsx: true });
    case "ts":
      return javascript({ typescript: true, jsx: false });
    case "tsx":
      return javascript({ typescript: true, jsx: true });
    case "json":
    case "jsonc":
      return json();
    case "css":
    case "scss":
    case "less":
      return css();
    case "html":
    case "htm":
      return html();
    case "md":
    case "markdown":
      return markdown();
    case "py":
      return python();
    case "rs":
      return rust();
    default:
      return null;
  }
}
