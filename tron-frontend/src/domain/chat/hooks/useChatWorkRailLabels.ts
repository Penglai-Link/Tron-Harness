import type { ChatWorkRailLabels } from "@tron/chat-core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export function useChatWorkRailLabels() {
  const { i18n, t } = useTranslation("pages");
  return useMemo<ChatWorkRailLabels>(() => {
    const number = new Intl.NumberFormat(i18n.language);
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
  }, [i18n.language, t]);
}
