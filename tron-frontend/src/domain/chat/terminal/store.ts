import { create } from "zustand";
import { requestApi } from "@/shared/api/httpClient";
export interface AgentTerminalInfo {
  terminalId: string;
  command: string;
  args: string[];
  cwd: string;
  running: boolean;
  exitCode: number | null;
}
export interface TerminalOutput {
  terminalId: string;
  cwd: string;
  running: boolean;
  exitCode: number | null;
  output: string;
  nextCursor: number;
  truncated: boolean;
  outputClosed: boolean;
}
interface Log {
  text: string;
  cursor: number;
  truncated: boolean;
  closed: boolean;
}
interface Session {
  tabs: string[];
  selected: string;
  terminals: AgentTerminalInfo[];
  logs: Record<string, Log>;
  error: string;
}
const empty = (): Session => ({
  tabs: [],
  selected: "",
  terminals: [],
  logs: {},
  error: "",
});
const savedTabs = (code: string): Pick<Session, "tabs" | "selected"> => {
  try {
    const value = JSON.parse(
      sessionStorage.getItem(`tron.agent-terminals.${code}`) || "null",
    );
    if (
      value &&
      Array.isArray(value.tabs) &&
      value.tabs.every((id: unknown) => typeof id === "string") &&
      typeof value.selected === "string"
    )
      return {
        tabs: [...new Set<string>(value.tabs)],
        selected: value.tabs.includes(value.selected)
          ? value.selected
          : (value.tabs[0] ?? ""),
      };
  } catch {
    /* unavailable storage */
  }
  return { tabs: [], selected: "" };
};
const persist = (code: string, value: Session) => {
  try {
    sessionStorage.setItem(
      `tron.agent-terminals.${code}`,
      JSON.stringify({ tabs: value.tabs, selected: value.selected }),
    );
  } catch {
    /* optional cache */
  }
};
export const useAgentTerminals = create<{
  sessions: Record<string, Session>;
  update: (code: string, fn: (value: Session) => Session) => void;
}>((set) => ({
  sessions: {},
  update: (code, fn) =>
    set((state) => {
      const next = fn(
        state.sessions[code] ?? { ...empty(), ...savedTabs(code) },
      );
      persist(code, next);
      return { sessions: { ...state.sessions, [code]: next } };
    }),
}));
export const terminalEndpoint = (session: string, id?: string) =>
  `/api/v1/chats/sessions/${encodeURIComponent(session)}/terminals${id ? `/${encodeURIComponent(id)}` : ""}`;
export function openAgentTerminal(session: string, id: string) {
  useAgentTerminals.getState().update(session, (value) => ({
    ...value,
    tabs: value.tabs.includes(id) ? value.tabs : [...value.tabs, id],
    selected: id,
  }));
  window.dispatchEvent(
    new CustomEvent("tron:open-agent-terminal", {
      detail: { sessionCode: session },
    }),
  );
}
export function openAgentTerminalFromLiveResult(
  session: string,
  event: {
    type?: string;
    toolCallName?: string;
    toolCallId?: string;
    runId?: string;
    content?: unknown;
  },
  seenCalls: Set<string>,
) {
  if (
    event.type !== "TOOL_CALL_RESULT" ||
    event.toolCallName !== "terminal_start" ||
    !event.runId ||
    !event.toolCallId
  )
    return false;
  let result: unknown = event.content;
  if (typeof result === "string") {
    try {
      result = JSON.parse(result);
    } catch {
      return false;
    }
  }
  if (!result || typeof result !== "object" || Array.isArray(result)) return false;
  const terminalId = (result as Record<string, unknown>).terminalId;
  if (typeof terminalId !== "string" || !terminalId) return false;
  const callKey = `${event.runId}:${event.toolCallId}`;
  if (seenCalls.has(callKey)) return false;
  seenCalls.add(callKey);
  if (seenCalls.size > 512) {
    const oldest = seenCalls.values().next().value;
    if (oldest) seenCalls.delete(oldest);
  }
  openAgentTerminal(session, terminalId);
  return true;
}
export function closeAgentTerminal(session: string, id: string) {
  useAgentTerminals.getState().update(session, (value) => {
    const tabs = value.tabs.filter((key) => key !== id);
    return {
      ...value,
      tabs,
      selected: value.selected === id ? (tabs[0] ?? "") : value.selected,
    };
  });
}
export async function refreshAgentTerminals(
  session: string,
  signal?: AbortSignal,
) {
  const terminals = await requestApi<AgentTerminalInfo[]>(
    terminalEndpoint(session),
    { signal },
  );
  useAgentTerminals
    .getState()
    .update(session, (value) => ({ ...value, terminals, error: "" }));
}
export async function stopAgentTerminal(session: string, id: string) {
  await requestApi(`POST ${terminalEndpoint(session, id)}/stop`);
  await refreshAgentTerminals(session);
}
export function appendTerminalOutput(
  session: string,
  id: string,
  result: TerminalOutput,
) {
  useAgentTerminals.getState().update(session, (value) => {
    const previous = value.logs[id];
    return {
      ...value,
      logs: {
        ...value.logs,
        [id]: {
          text: ((previous?.text ?? "") + result.output).slice(-262144),
          cursor: result.nextCursor,
          truncated: Boolean(
            previous?.truncated ||
            result.truncated ||
            (previous?.text.length ?? 0) + result.output.length > 262144,
          ),
          closed: result.outputClosed,
        },
      },
      terminals: value.terminals.map((item) =>
        item.terminalId === id
          ? { ...item, running: result.running, exitCode: result.exitCode }
          : item,
      ),
    };
  });
}
