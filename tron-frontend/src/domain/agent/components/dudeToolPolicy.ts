import type { PlatformToolRecord, ToolAssignmentMode } from "../model";

export function dudeToolsForRule(tools: PlatformToolRecord[], mode: ToolAssignmentMode, categories: string[], selectedTools: string[] = []): PlatformToolRecord[] {
  const byCode = new Map(tools.map((tool) => [tool.toolCode, tool]));
  const result = new Map<string, PlatformToolRecord>();
  for (const tool of tools) {
    if (tool.assignable === false) continue;
    const selected = mode === "all_platform_tools"
      || (mode === "all_read_only_tools" && tool.operationKind === "query")
      || (mode === "selected_categories" && (categories.includes(tool.category?.code ?? "") || (categories.includes("readonly") && tool.operationKind === "query")))
      || (mode === "selected_tools" && selectedTools.includes(tool.toolCode));
    if (!selected) continue;
    const actual = tool.dudeAssignable === false ? byCode.get(tool.dudeReplacementToolCode ?? "") : tool;
    if (actual && actual.assignable !== false && actual.dudeAssignable !== false) result.set(actual.toolCode, actual);
  }
  return [...result.values()];
}

export function hasDudeToolAlias(tool: PlatformToolRecord): boolean {
  return Boolean(tool.dudeReplacementToolCode);
}
