import { matchPath } from "react-router";
import type { i18n } from "i18next";
import { appRoutes } from "@/router/routes";
import type { TraceDetail } from "../model/Trace";

export function triggerLabel(value: string | undefined, i18n: i18n, category: "Page" | "Agent" = "Page"): string {
  if (!value) return "—";
  if (category === "Agent") return value;
  const path = value.replace(/^page /, "");
  if (!path.startsWith("/")) {
    const route = appRoutes.find((route) => route.title === path || route.pageId === path);
    return route?.titleTranslationKey ? String(i18n.t(route.titleTranslationKey)) : value;
  }
  const pathname = path.split(/[?#]/)[0];
  const route = appRoutes.find((route) => matchPath({ path: route.route.split(/[?#]/)[0], end: true }, pathname));
  if (pathname === "/resource-management/files" || pathname.startsWith("/resource-management/files/")) return String(i18n.t("observability:filesTrigger"));
  const section = pathname.split("/")[1];
  const keys: Record<string, string> = { chats:"chatTrigger", apps:"appsTrigger", resources:"filesTrigger", files:"filesTrigger" };
  if (keys[section]) return String(i18n.t(`observability:${keys[section]}`));
  return route?.titleTranslationKey ? String(i18n.t(route.titleTranslationKey)) : route?.title ?? path;
}

/** Keep the selected entry and all descendants, with metrics for that subtree only. */
export function triggerDetail(detail: TraceDetail, rootId: string | null): TraceDetail {
  const root = detail.spans.find((span) => span.spanId === rootId);
  if (!root) return detail;
  const children = new Map<string, string[]>();
  for (const span of detail.spans) {
    if (!span.parentSpanId) continue;
    const siblings = children.get(span.parentSpanId);
    if (siblings) siblings.push(span.spanId);
    else children.set(span.parentSpanId, [span.spanId]);
  }
  const ids = new Set<string>();
  const pending = [root.spanId];
  while (pending.length) {
    const id = pending.pop()!;
    if (ids.has(id)) continue;
    ids.add(id);
    for (const child of children.get(id) ?? []) pending.push(child);
  }
  const spans = detail.spans.filter((span) => ids.has(span.spanId));
  const models = spans.filter((span) => span.name === "agent.model");
  const sum = (key: string) => models.reduce((total, span) => total + (Number(span.attributes[key]) || 0), 0);
  const running = spans.some((span) => span.status === "Running");
  const end = spans.reduce((end, span) => Math.max(end, span.startedAt + span.durationMs), running ? Date.now() : 0);
  return { spans, trace: { ...detail.trace, rootSpanId:root.spanId,
    trigger:root.attributes["tron.dude.name"] || root.attributes["tron.agent.name"] || root.attributes["tron.dude.code"] || root.attributes["tron.agent.code"] || root.attributes["url.path"] || root.name,
    workspaceCode:root.attributes["tron.workspace.code"] || detail.trace.workspaceCode,
    sessionCode:root.attributes["tron.session.code"] || detail.trace.sessionCode,
    agentCode:root.attributes["tron.agent.code"] || detail.trace.agentCode,
    startedAt:root.startedAt, endedAt:running ? null : end,
    durationMs:Math.max(0,end-root.startedAt), status:running ? "Running" : root.status,
    spanCount:spans.length, errorCount:spans.filter((span) => span.status === "Error").length,
    modelCount:models.length, toolCount:spans.filter((span) => span.name === "agent.tool").length,
    inputTokens:sum("gen_ai.usage.input_tokens"), outputTokens:sum("gen_ai.usage.output_tokens"), cachedInputTokens:sum("gen_ai.usage.cached_input_tokens"),
  } };
}
