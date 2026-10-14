import { useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { ChatClientConfig } from "@tron/chat-core";
import { requestApi } from "@/shared/api/httpClient";
import { CAPABILITY_RESOURCES_CHANGED } from "@/domain/skill/api/skillApi";
import { useChatFeedback } from "./useChatFeedback";

interface InputSuggestionsResponse {
  agentCode: string;
  skills: Array<{ name: string; description: string }>;
  commands: string[];
}

/** Server resolves persisted Agent ownership and returns only effective grants. */
export function useChatInputSuggestions(agentCode?: string, sessionCode?: string, dudeCode?: string, workspaceCode?: string | null) {
  const { t, i18n } = useTranslation("pages");
  const feedback = useChatFeedback();
  const query = useQuery({
    queryKey: ["chat-input-suggestions", agentCode, sessionCode, dudeCode, workspaceCode, i18n.resolvedLanguage],
    queryFn: ({ signal }) => requestApi<InputSuggestionsResponse>("GET /api/v1/chats/input-suggestions", {
      queryParams: { agentCode, sessionCode, workspaceCode: workspaceCode ?? undefined },
      headers: dudeCode ? { "X-Dude-Code": dudeCode } : undefined, signal,
    }),
    staleTime: 0,
    enabled: Boolean(workspaceCode || dudeCode),
  });
  const { refetch } = query;
  useEffect(() => {
    const refresh = () => { void refetch(); };
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, refresh);
    return () => window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, refresh);
  }, [refetch]);
  useEffect(() => {
    if (query.error) feedback.error(query.error.message, "Chat suggestions failed");
  }, [query.error, feedback.error, t]);
  return useMemo<Pick<ChatClientConfig, "inputSuggestions" | "inputSuggestionLabels" | "skillSuggestionTriggers">>(() => ({
    skillSuggestionTriggers: ["$"],
    inputSuggestions: query.error ? [] : [
      ...(query.data?.commands ?? []).map(name => ({ kind: "command" as const, name, description: t("chat.management.composer.goalSuggestion") })),
      ...(query.data?.skills ?? []).map(skill => ({ ...skill, kind: "skill" as const })),
    ],
    inputSuggestionLabels: {
      commands: t("chat.management.composer.commands"), skills: t("chat.management.composer.skills"),
      commandsAndSkills: t("chat.management.composer.commandsAndSkills"),
    },
  }), [query.data, query.error, t]);
}
