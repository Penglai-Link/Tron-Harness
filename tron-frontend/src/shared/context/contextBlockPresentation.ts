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
