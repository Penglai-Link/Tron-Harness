import { useMemo, type MouseEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { useToast } from "@/shared/ui/Toast";

interface MarkdownLinkProps {
  children?: ReactNode;
  href?: string;
  onOpenInternal?: (url: string) => void;
}

function normalizeHttpUrl(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const candidate = trimmed.startsWith("//") ? `https:${trimmed}` : trimmed;
  if (!/^https?:\/\//i.test(candidate)) {
    return null;
  }

  return candidate;
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

export function MarkdownLink({ children, href, onOpenInternal }: MarkdownLinkProps) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const bridge = useMemo(() => new DesktopBridge(), []);
  const url = normalizeHttpUrl(href ?? "");

  function openExternal() {
    if (!url) {
      return;
    }

    if (!bridge.isDesktop()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }

    // 桌面端：走 IPC 让主进程打开系统浏览器
    // fire-and-forget：主进程立即返回，不等浏览器打开，
    // 避免渲染进程后续处理抢回焦点导致浏览器窗口层级变低
    void bridge.openTargetPage({ mode: "browser", url }).catch(() => {
      toast.error(t("chat.markdownLink.openExternalFailed"));
    });
  }

  function openInternal() {
    if (!url) {
      return;
    }

    if (bridge.isDesktop() && onOpenInternal) {
      onOpenInternal(url);
      return;
    }

    window.open(url, "_blank", "noopener,noreferrer");
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
    return <a href={href}>{children}</a>;
  }

  function handleClick(event: MouseEvent<HTMLAnchorElement>) {
    event.preventDefault();
    if (event.ctrlKey || event.metaKey) {
      openExternal();
    } else {
      openInternal();
    }
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger
        render={<a href={url} onClick={handleClick}>{children}</a>}
      />
      <ContextMenuContent>
        <ContextMenuItem onClick={openInternal}>
          {t("chat.markdownLink.openInternal")}
        </ContextMenuItem>
        <ContextMenuItem onClick={() => setTimeout(openExternal, 0)}>
          {t("chat.markdownLink.openExternal")}
        </ContextMenuItem>
        <ContextMenuItem onClick={copyLink}>
          {t("chat.markdownLink.copyLink")}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
