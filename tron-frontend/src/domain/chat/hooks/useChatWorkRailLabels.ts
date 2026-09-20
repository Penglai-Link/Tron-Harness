import { toolCommandLinePreview, type ChatWorkRailLabels } from "@tron/chat-core";
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
    const resolveToolName = (runtimeName: string, args: unknown) => {
      const commandLine = toolCommandLinePreview(runtimeName, args);
      if (commandLine) {
        return t("chat.management.tool.commandSummary", { command: commandLine });
      }
      if (runtimeName === "discovery_expand_tool_domain") {
        const payload = typeof args === "string"
          ? (() => { try { return JSON.parse(args) as { domainCode?: unknown }; } catch { return {}; } })()
          : args as { domainCode?: unknown } | null;
        const domainCode = typeof payload?.domainCode === "string" ? payload.domainCode : "";
        return catalog.domains.get(domainCode) ?? (domainCode || runtimeName);
      }
      return catalog.tools.get(runtimeName) ?? runtimeName;
    };
    return {
      ariaLabel: t("chat.workRail.ariaLabel"),
      preparing: t("chat.workRail.preparing"),
      responding: t("chat.workRail.responding"),
      thinking: t("chat.workRail.thinking"),
      workedFor: (duration) => t("chat.workRail.workedFor", { duration }),
      workingFor: (duration) => t("chat.workRail.workingFor", { duration }),
      failed: t("chat.workRail.failed"),
      askingQuestions: (count) => t("chat.workRail.askingQuestions", { count }),
      askedQuestions: (count) => t("chat.workRail.askedQuestions", { count }),
      waitingForAnswer: t("chat.workRail.waitingForAnswer"),
      skipped: t("chat.workRail.skipped"),
      tool: {
        ariaLabel: t("chat.management.tool.stepsAria"),
        completed: (name) => t("chat.management.tool.completed", { name }),
        running: (name) => t("chat.management.tool.running", { name }),
        domainExpanded: (name) => t("chat.management.tool.domainExpanded", { name }),
        domainExpanding: (name) => t("chat.management.tool.domainExpanding", { name }),
        group: (count) => t("chat.management.tool.ranCommands", { count }),
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
