import { contextBlockName } from "@/shared/context/contextBlockPresentation";
import { isRawToolNameOverride, toolCommandLinePreview, toolDisplayName, type ToolActionSummary } from "@personai/agent-ui-chat-core";
import type { ChatWorkRailLabels } from "@tron/chat-core";
import { fetchToolDomains, listTools } from "@/domain/agent/api";
import { listToolDomainsMock, listToolsMock } from "@/domain/agent/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

export function useChatWorkRailLabels() {
  const { i18n, t } = useTranslation("pages");
  const [catalog, setCatalog] = useState(() => ({
    domains: new Map<string, string>(),
    tools: new Map<string, string>(),
  }));

  useEffect(() => {
    const controller = new AbortController();
    const load = getServiceDataSource("agent") === "mock"
      ? Promise.resolve([listToolDomainsMock(), listToolsMock()] as const)
      : Promise.all([fetchToolDomains(controller.signal), listTools({}, controller.signal)]);
    void load.then(([domains, tools]) => {
      if (controller.signal.aborted) return;
      setCatalog({
        domains: new Map(domains.map((domain) => [domain.domainCode, domain.name])),
        tools: new Map(tools.map((tool) => [tool.toolName, tool.name])),
      });
    }).catch(() => {
      if (!controller.signal.aborted) setCatalog({ domains: new Map(), tools: new Map() });
    });
    return () => controller.abort();
  }, [i18n.language]);

  return useMemo<ChatWorkRailLabels>(() => {
    const number = new Intl.NumberFormat(i18n.language);
    const groupSummary = (summary: ToolActionSummary) => {
      const parts: string[] = [];
      if (summary.commands) parts.push(t("chat.management.tool.actionSummary.commands", { count: summary.commands }));
      if (summary.editedFiles) parts.push(t("chat.management.tool.actionSummary.editedFiles", { count: summary.editedFiles }));
      if (summary.editOperations) parts.push(t("chat.management.tool.actionSummary.editOperations", { count: summary.editOperations }));
      if (summary.searches) parts.push(t("chat.management.tool.actionSummary.searches", { count: summary.searches }));
      if (summary.reads) parts.push(t("chat.management.tool.actionSummary.reads", { count: summary.reads }));
      if (summary.otherActions) parts.push(t("chat.management.tool.actionSummary.otherActions", { count: summary.otherActions }));
      return i18n.language.startsWith("zh") ? parts.join("，") : new Intl.ListFormat(i18n.language, { style: "short", type: "conjunction" }).format(parts);
    };
    const resolveToolName = (runtimeName: string, args: unknown) => {
      const commandLine = toolCommandLinePreview(runtimeName, args);
      if (commandLine) {
        return t("chat.management.tool.commandSummary", { command: commandLine });
      }
      // A catalogue name is only useful when it is actually a localized label:
      // for the tools whose catalogue entry still equals the raw tool id, fall
      // back to the presenter title in the active language.
      const catalogName = catalog.tools.get(runtimeName);
      const directoryName = catalogName && !isRawToolNameOverride(runtimeName, catalogName, args) ? catalogName : undefined;
      const name = directoryName ?? toolDisplayName(runtimeName, args, undefined, i18n.language);
      if (runtimeName === "rag_search") {
        const payload = typeof args === "string"
          ? (() => { try { return JSON.parse(args) as { query?: unknown }; } catch { return {}; } })()
          : args as { query?: unknown } | null;
        const query = typeof payload?.query === "string" ? payload.query.trim().replace(/\s+/g, " ") : "";
        if (query) return t("chat.management.tool.searchSummary", { name, query });
      }
      return name;
    };
    // Domain labels never render the raw domain code when the catalogue has a
    // distinct localized name for it.
    const resolveDomainName = (name: string) => {
      const catalogName = catalog.domains.get(name);
      return catalogName && !isRawToolNameOverride(name, catalogName) ? catalogName : name;
    };
    return {
      ariaLabel: t("chat.workRail.ariaLabel"),
      preparing: t("chat.workRail.preparing"),
      responding: t("chat.workRail.responding"),
      thinking: t("chat.workRail.thinking"),
      working: t("chat.workRail.working"),
      workedFor: (duration) => t("chat.workRail.workedFor", { duration }),
      workingFor: (duration) => t("chat.workRail.workingFor", { duration }),
      failed: t("chat.workRail.failed"),
      stopped: t("chat.workRail.stopped"),
      askingQuestions: (count) => t("chat.workRail.askingQuestions", { count }),
      askedQuestions: (count) => t("chat.workRail.askedQuestions", { count }),
      waitingForAnswer: t("chat.workRail.waitingForAnswer"),
      skipped: t("chat.workRail.skipped"),
      tool: {
        ariaLabel: t("chat.management.tool.stepsAria"),
        completed: (name) => t("chat.management.tool.completed", { name }),
        running: (name) => t("chat.management.tool.running", { name }),
        domainExpanded: (name) => t("chat.management.tool.domainExpanded", { name: resolveDomainName(name) }),
        domainExpanding: (name) => t("chat.management.tool.domainExpanding", { name: resolveDomainName(name) }),
        group: (count) => t("chat.management.tool.ranCommands", { count }),
        groupSummary,
        statusDone: t("chat.management.tool.badgeDone"),
        statusRunning: t("chat.management.tool.badgeRunning"),
        parameters: t("chat.management.tool.parameters"),
        result: t("chat.management.tool.result"),
        log: t("chat.management.tool.log"),
        noResult: t("chat.management.tool.noResult"),
        waitingActivity: t("chat.management.tool.waitingActivity"),
        copyParameters: t("chat.management.tool.copyParameters"),
        copyResult: t("chat.management.tool.copyResult"),
        reasoningCaptured: t("chat.management.reasoning.captured"),
        reasoningThinking: t("chat.management.reasoning.thinking"),
        reasoningAria: t("chat.management.reasoning.aria"),
        emptyReasoning: t("chat.management.reasoning.empty"),
        resolveName: resolveToolName,
      },
      notices: {
        compactionBlock: (block) => t(block.kind === "summary" ? "chat.workRail.compactionSummary" : "chat.workRail.compactionBlock", {
          name: contextBlockName(block.id, (key, options) => t(key, options)),
          before: number.format(block.beforeTokens), after: number.format(block.afterTokens),
        }),
        compactionNoChanges: t("chat.workRail.compactionNoChanges"),
        compactionDetailsUnavailable: t("chat.workRail.compactionDetailsUnavailable"),
        mcpUnavailable: t("chat.shortTermMemory.contextPanel.runtimeNotices.mcp_unavailable.title"),
        presentationIncomplete: t("chat.shortTermMemory.contextPanel.runtimeNotices.presentation_incomplete.title"),
        presentationIncompleteDetail: t("chat.shortTermMemory.contextPanel.runtimeNotices.presentation_incomplete.detail"),
        compactionRequested: t("chat.shortTermMemory.contextPanel.runtimeNotices.compaction_requested.title"),
        compactionStarted: t("chat.shortTermMemory.contextPanel.runtimeNotices.compaction_started.title"),
        compactionCompleted: t("chat.shortTermMemory.contextPanel.runtimeNotices.compaction_completed.title"),
        compactionFailed: t("chat.shortTermMemory.contextPanel.runtimeNotices.compaction_failed.title"),
        compactionInterrupted: t("chat.workRail.compactionInterrupted"),
        toolSchemaOverBudget: t("chat.shortTermMemory.contextPanel.runtimeNotices.tool_schema_over_budget.title"),
        toolSchemaBudget: (actual, limit, count) => t("chat.shortTermMemory.contextPanel.runtimeNotices.tool_schema_over_budget.detailWithBudget", {
          actual: number.format(actual), limit: number.format(limit), count: number.format(count),
        }),
        toolSchemaBudgetHint: t("chat.shortTermMemory.contextPanel.runtimeNotices.tool_schema_over_budget.detail"),
      },
    };
  }, [catalog, i18n.language, t]);
}

export function useChatHistoryLabels() {
  const { i18n } = useTranslation("pages");
  return useMemo(() => i18n.language.startsWith("zh") ? {
    loading: "正在加载历史…", loadOlder: "加载更早的对话", step: "步骤", attempt: "尝试",
    statuses: { Running: "执行中", Waiting: "等待输入", Completed: "已完成", Interrupted: "已中断", Failed: "失败", Unknown: "结果未知", Planned: "待执行", Approved: "已批准" },
  } : {
    loading: "Loading history…", loadOlder: "Load earlier turns", step: "Step", attempt: "Attempt",
    statuses: { Running: "Running", Waiting: "Waiting", Completed: "Completed", Interrupted: "Interrupted", Failed: "Failed", Unknown: "Result unknown", Planned: "Planned", Approved: "Approved" },
  }, [i18n.language]);
}
