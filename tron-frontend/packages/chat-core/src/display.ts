export function sessionAgentTagLabel(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") return "MAIN";
  return normalized.toUpperCase();
}
