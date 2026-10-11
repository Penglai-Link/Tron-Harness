import { Fragment } from "react";

/** Literal, case-insensitive matching; log text remains escaped React content. */
export function LogHighlight({ text, keyword = "" }: { text: string; keyword?: string }) {
  if (!keyword) return <>{text}</>;
  const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const parts = text.split(new RegExp(`(${escaped})`, "gi"));
  return <>{parts.map((part, index) => index % 2
    ? <mark key={index} className="rounded-sm bg-amber-200 px-0.5 text-gray-950 dark:bg-amber-400/30 dark:text-amber-100">{part}</mark>
    : <Fragment key={index}>{part}</Fragment>)}</>;
}
