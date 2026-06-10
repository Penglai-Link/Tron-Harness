export interface ChatModelAgentSourceLike {
  optionCode?: string | null;
  source?: "main-agent" | "system-agent" | string | null;
}

function normalizedAgentCode(value: string | null | undefined) {
  const normalized = value?.trim().toLowerCase();
  return normalized ? normalized : "";
}

function systemAgentCodeFromOptionCode(optionCode: string | null | undefined) {
  const match = optionCode?.trim().match(/^system:([^:]+):/i);
  return match?.[1]?.trim() ?? "";
}

export function inferChatAgentCodeFromModel(
  model: ChatModelAgentSourceLike | null | undefined,
  fallbackAgentCode: string,
) {
  if (model?.source === "system-agent") {
    const systemAgentCode = systemAgentCodeFromOptionCode(model.optionCode);
    if (systemAgentCode) return systemAgentCode;
  }

  if (model?.source === "main-agent") {
    return "main-agent";
  }

  return normalizedAgentCode(fallbackAgentCode) || "main-agent";
}
