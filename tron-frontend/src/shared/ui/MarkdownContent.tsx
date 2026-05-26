import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useTranslation } from "react-i18next";

import { cn } from "@/shared/lib/utils";

interface MarkdownContentProps {
  ariaLabel?: string;
  className?: string;
  content?: string | null;
  emptyMessage?: string;
}

export function MarkdownContent({
  ariaLabel,
  className,
  content,
  emptyMessage,
}: MarkdownContentProps) {
  const { t } = useTranslation("common");
  const value = content?.trim();

  if (!value) {
    return (
      <div
        aria-label={ariaLabel}
        className={cn(
          "min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)]",
          className,
        )}
      >
        <span className="text-[var(--color-on-surface-variant)]">
          {emptyMessage ?? t("content.none")}
        </span>
      </div>
    );
  }

  return (
    <div
      aria-label={ariaLabel}
      className={cn(
        "min-h-6 py-1 text-sm leading-7 text-[var(--color-on-surface)]",
        "[&_a]:text-[var(--color-primary)] [&_a]:underline-offset-2 hover:[&_a]:underline",
        "[&_blockquote]:mb-3 [&_blockquote]:border-l-2 [&_blockquote]:border-[var(--color-outline-variant)] [&_blockquote]:pl-4 [&_blockquote]:text-[var(--color-on-surface-variant)]",
        "[&_code]:font-mono [&_code]:text-[13px]",
        "[&_h1]:mb-4 [&_h1]:text-2xl [&_h1]:font-semibold [&_h1]:leading-tight",
        "[&_h2]:mb-3 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:leading-tight",
        "[&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold [&_h3]:leading-tight",
        "[&_hr]:my-4 [&_hr]:border-[var(--color-outline-variant)]",
        "[&_li]:mb-1",
        "[&_ol]:mb-3 [&_ol]:list-decimal [&_ol]:pl-5",
        "[&_p]:mb-3 [&_p:last-child]:mb-0",
        "[&_p_code]:rounded [&_p_code]:bg-[var(--color-surface-container-lowest)] [&_p_code]:px-1.5 [&_p_code]:py-0.5",
        "[&_pre]:mb-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-[var(--color-surface-container-lowest)] [&_pre]:p-3",
        "[&_pre_code]:bg-transparent [&_pre_code]:p-0",
        "[&_table]:mb-3 [&_table]:w-full [&_table]:border-collapse [&_table]:overflow-hidden [&_table]:rounded-lg",
        "[&_tbody_tr]:border-b [&_tbody_tr]:border-[var(--color-outline-variant)] [&_tbody_tr:last-child]:border-b-0",
        "[&_td]:px-3 [&_td]:py-2 [&_td]:align-top",
        "[&_th]:border-b [&_th]:border-[var(--color-outline-variant)] [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:font-semibold",
        "[&_ul]:mb-3 [&_ul]:list-disc [&_ul]:pl-5",
        className,
      )}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{value}</ReactMarkdown>
    </div>
  );
}
