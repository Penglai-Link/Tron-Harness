export type ContextBlockUpdateMode = "Fixed" | "Append" | "Replace" | "Unspecified";

/** Localized by the provider; custom strategy IDs need no client-side mapping. */
export interface ContextCompressionMetadata {
  id: string;
  displayName: string;
  description: string;
}

const contextBlockLabelKeys: Record<string, string> = {
  "framework.instructions": "systemPromptName",
  seed_prompt_prefix: "systemPromptName",
  application_context: "applicationContextName",
  page_context: "pageContextName",
  conversation_history_summary: "conversationSummaryName",
  current_session_checkpoint: "conversationSummaryName", // Historical assembly reports only.
  current_session_working_state: "workingStateName",
  "framework.history": "sessionHistoryName",
  current_session_history: "sessionHistoryName",
  runtime_identity: "runtimeIdentityName",
  "skills.catalog": "skillsCatalogName",
  "tool_domains": "toolDomainsName",
  "skills.guidance": "skillsGuidanceName",
  "agent.directory-guidance": "directoryGuidanceName",
  "dude.guidance": "dudeGuidanceName",
  "a2ui.guidance": "a2uiGuidanceName",
  long_term_memory: "longTermMemoryName",
  knowledge_base: "knowledgeBaseName",
  agent_directory: "agentDirectoryName",
  dude_onboarding: "dudeOnboardingName",
  dude_environment: "dudeEnvironmentName",
  dude_wakeup: "dudeWakeupName",
};

/** Shared product names for configuration sources and assembled context reports. */
export function contextBlockName(
  id: string,
  translate: (key: string, options?: Record<string, unknown>) => string,
  fallback = id,
): string {
  const key = contextBlockLabelKeys[id];
  return key ? translate(`chat.shortTermMemory.contextPanel.${key}`) : fallback;
}
