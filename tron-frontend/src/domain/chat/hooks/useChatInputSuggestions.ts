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
export function useChatInputSuggestions(agentCode?: string, sessionCode?: string, dudeCode?: string, workspaceCode?: string | null, knowledgeBaseId?: string) {
  const { t, i18n } = useTranslation("pages");
  const feedback = useChatFeedback();
  const query = useQuery({
    queryKey: ["chat-input-suggestions", agentCode, sessionCode, dudeCode, workspaceCode, knowledgeBaseId, i18n.resolvedLanguage],
    queryFn: ({ signal }) => requestApi<InputSuggestionsResponse>("GET /api/v1/chats/input-suggestions", {
      queryParams: { agentCode, sessionCode, workspaceCode: workspaceCode ?? undefined },
      headers: { ...(dudeCode ? { "X-Dude-Code": dudeCode } : {}), ...(knowledgeBaseId ? { "X-Knowledge-Base-Code": knowledgeBaseId } : {}) }, signal,
    }),
    // Suggestions change rarely; capability changes force a refetch through
    // CAPABILITY_RESOURCES_CHANGED, so a short cache window avoids refetching
    // on every composer remount.
    staleTime: 30_000,
    enabled: Boolean(workspaceCode || dudeCode || knowledgeBaseId),
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
      ...(query.data?.commands ?? []).map(name => ({ kind: "command" as const, name, description: t(`chat.management.composer.${name}Suggestion`, { defaultValue: "" }) })),
      ...(query.data?.skills ?? []).map(skill => ({ ...skill, kind: "skill" as const })),
    ],
    inputSuggestionLabels: {
      commands: t("chat.management.composer.commands"), skills: t("chat.management.composer.skills"),
      commandsAndSkills: t("chat.management.composer.commandsAndSkills"),
    },
  }), [query.data, query.error, t]);
}
