import { useMemo, type MouseEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useToast } from "@/components/ui/toast";

interface MarkdownLinkProps {
  children?: ReactNode;
  href?: string;
  onOpenInternal?: (url: string) => void;
  onOpenExternal?: (url: string) => void;
}

// 视为链接尾部杂质的标点集合（ASCII 与全角）
const JUNK_CHAR_CLASS = ".,;:!?)\\]}\"'’”»›…～—·，。；：！？）】》〉」』〕｝";
const TRAILING_JUNK_PATTERN = new RegExp(`[${JUNK_CHAR_CLASS}]+$`);
const JUNK_ONLY_PATTERN = new RegExp(`^[${JUNK_CHAR_CLASS}]+$`);

function toHttpUrl(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const candidate = trimmed.startsWith("//") ? `https:${trimmed}` : trimmed;
  if (/^https?:\/\//i.test(candidate)) {
    // 主机名不能为空，避免剔除杂质后残留 https:// 这类残缺链接
    return /^https?:\/\/[^/?#]+/i.test(candidate) ? candidate : null;
  }

  if (/^www\./i.test(candidate)) {
    return `https://${candidate}`;
  }

  return null;
}

const URL_SUBSTRING_PATTERN = /(?:https?:\/\/|www\.)[^\s<>"'`]+/i;

// 剔除链接尾部杂质标点：
// - 含 ? 或 # 时，末尾字符属于查询参数或锚点，是合法 URL 组成部分，原样保留
// - 末尾是被 URL 编码的标点（如 %E3%80%82 即 。）或明文标点时剔除，
//   剔除的字符（junk）交由调用方按普通文本展示
interface StrippedCandidate {
  cleaned: string;
  junk: string;
}

function stripTrailingJunk(value: string): StrippedCandidate {
  if (/[?#]/.test(value)) {
    return { cleaned: value, junk: "" };
  }

  let result = value;
  for (;;) {
    const encoded = result.match(/(?:%[0-9A-Fa-f]{2})+$/);
    if (encoded && encoded.index !== undefined) {
      try {
        const decoded = decodeURIComponent(encoded[0]);
        // 仅当整段编码内容都是杂质标点时才剔除，避免误伤正常参数值
        if (JUNK_ONLY_PATTERN.test(decoded)) {
          result = result.slice(0, encoded.index);
          continue;
        }
      } catch {
        // 非法百分号编码，按普通字符处理
      }
    }

    const stripped = result.replace(TRAILING_JUNK_PATTERN, "");
    if (stripped === result) {
      return { cleaned: result, junk: value.slice(result.length) };
    }
    result = stripped;
  }
}

interface ParsedLink {
  url: string | null;
  before: string;
  after: string;
  /** 保留 href 中原始形态的尾部杂质（可能是 %XX 编码） */
  junkRaw: string;
  /** 展示用的杂质文本（编码已解码，如 %E3%80%82 → 。） */
  junkDisplay: string;
}

// markdown 渲染时会将 href 中的全角标点做百分号编码，而链接文字保留明文，
// 因此展示与文字匹配都要用解码后的形态
function decodeJunk(junk: string): string {
  if (!junk) {
    return "";
  }

  try {
    return decodeURIComponent(junk);
  } catch {
    return junk;
  }
}

function parseLink(value: string): ParsedLink {
  const trimmed = value.trim();
  const empty: ParsedLink = { url: null, before: "", after: "", junkRaw: "", junkDisplay: "" };
  if (!trimmed) {
    return empty;
  }

  const whole = stripTrailingJunk(trimmed);
  const wholeUrl = toHttpUrl(whole.cleaned);
  if (wholeUrl) {
    // 整串即合法链接：剔除的尾部杂质解码后放入 after，按普通文本展示
    const junkDisplay = decodeJunk(whole.junk);
    return { url: wholeUrl, before: "", after: junkDisplay, junkRaw: whole.junk, junkDisplay };
  }

  const match = trimmed.match(URL_SUBSTRING_PATTERN);
  if (!match || match.index === undefined) {
    return empty;
  }

  const stripped = stripTrailingJunk(match[0]);
  const url = toHttpUrl(stripped.cleaned);
  if (!url) {
    return empty;
  }

  const junkDisplay = decodeJunk(stripped.junk);
  const remainder = trimmed.slice(
    match.index + stripped.cleaned.length + stripped.junk.length,
  );

  return {
    url,
    before: trimmed.slice(0, match.index),
    // 杂质部分用解码后的形态展示，其后才是剩余普通文本
    after: junkDisplay + remainder,
    junkRaw: stripped.junk,
    junkDisplay,
  };
}

async function copyPlainText(value: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  document.body.removeChild(textarea);
  if (!copied) {
    throw new Error("Clipboard copy failed");
  }
}

export function MarkdownLink({
  children,
  href,
  onOpenInternal,
  onOpenExternal,
}: MarkdownLinkProps) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { url, before, after, junkRaw, junkDisplay } = useMemo(
    () => parseLink(href ?? ""),
    [href],
  );

  // 链接文字若以被剔除的杂质结尾（明文或编码形态），则把杂质从超链接文本中分离
  function stripJunkSuffix(text: string): string | null {
    if (junkDisplay && text.endsWith(junkDisplay)) {
      return text.slice(0, text.length - junkDisplay.length);
    }
    if (junkRaw && text.endsWith(junkRaw)) {
      return text.slice(0, text.length - junkRaw.length);
    }
    return null;
  }

  function resolveAnchorContent(): ReactNode {
    if (typeof children === "string") {
      const stripped = stripJunkSuffix(children);
      if (stripped !== null) {
        return stripped.length > 0 ? stripped : url;
      }
    } else if (Array.isArray(children) && children.length > 0) {
      const last = children[children.length - 1];
      if (typeof last === "string") {
        const stripped = stripJunkSuffix(last);
        if (stripped !== null) {
          return stripped.length > 0
            ? [...children.slice(0, -1), stripped]
            : children.slice(0, -1);
        }
      }
    }

    return children ?? url;
  }

  function openExternal() {
    if (!url) {
      return;
    }

    onOpenExternal?.(url);
  }

  function openInternal() {
    if (!url) {
      return;
    }

    onOpenInternal?.(url);
  }

  function copyLink() {
    if (!url) {
      return;
    }

    void copyPlainText(url)
      .then(() => toast.success(t("chat.markdownLink.copied")))
      .catch(() => toast.error(t("chat.markdownLink.copyFailed")));
  }

  if (!url) {
    // 提取不到合法超链接时，整体按普通文本展示，避免非法链接被点击
    return <span>{children ?? href}</span>;
  }

  function handleClick(event: MouseEvent<HTMLAnchorElement>) {
    event.preventDefault();
    if (onOpenInternal && !event.ctrlKey && !event.metaKey) {
      openInternal();
    } else {
      openExternal();
    }
  }

  return (
    <>
      {before ? <span>{before}</span> : null}
      <ContextMenu>
        <ContextMenuTrigger
          render={<a href={url} onClick={handleClick}>{resolveAnchorContent()}</a>}
        />
        <ContextMenuContent>
          {onOpenInternal ? (
            <ContextMenuItem onClick={openInternal}>
              {t("chat.markdownLink.openInternal")}
            </ContextMenuItem>
          ) : null}
          <ContextMenuItem onClick={() => setTimeout(openExternal, 0)}>
            {t("chat.markdownLink.openExternal")}
          </ContextMenuItem>
          <ContextMenuItem onClick={copyLink}>
            {t("chat.markdownLink.copyLink")}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
      {after ? <span>{after}</span> : null}
    </>
  );
}
