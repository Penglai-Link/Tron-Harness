/** Emit the public batch envelope used by the real two-tool gateway. */
export function gatewayArguments(name: string, value: unknown): unknown {
  const args = value as Record<string, unknown>;
  if (name === "tool_search") return Array.isArray(args.items) ? args : { items: [args] };
  if (name === "tool_execute") {
    if (["chat_ask_user", "ask_user", "a2ui_publish", "a2ui_request_input", "worktree_create", "worktree_select", "git_switch_branch", "word_create", "word_replace_text", "excel_write", "fs_write_text", "fs_apply_patch", "fs_apply_unified_patch", "terminal_write", "command_exec_write", "system_task_write"].includes(String(args.tool))) return value;
    const arguments_ = args.arguments as Record<string, unknown>;
    return { ...args, arguments: Array.isArray(arguments_?.items) ? arguments_ : { items: [arguments_] } };
  }
  return value;
}

/** One-operation fixtures inspect the result inside the public batch envelope. */
export function singleGatewayResult(text: string): string {
  let value: { results?: Array<{ index: number; result: unknown }> };
  try { value = JSON.parse(text); } catch { return text; }
  if (!Array.isArray(value.results)) return text;
  if (value.results.length !== 1 || value.results[0]?.index !== 0) throw new Error("Expected a single gateway result");
  const result = value.results[0].result;
  return typeof result === "string" ? result : JSON.stringify(result);
}
