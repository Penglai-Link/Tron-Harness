import type { ChatAguiStreamEvent } from "./ChatModel";

export interface ChatReplayRun {
  runCode: string;
  supportsSteer: boolean | null;
}

interface OrderedProtocolEvent {
  event: ChatAguiStreamEvent;
  originalIndex: number;
  timestamp: number | null;
}

interface ReplayRunLifecycle extends ChatReplayRun {
  firstOrder: number;
  startOrder: number | null;
  terminal: boolean;
}

function eventRunCode(event: ChatAguiStreamEvent) {
  const value = event.runId ?? event.runCode;
  return typeof value === "string" ? value.trim() : "";
}

function eventThreadId(event: ChatAguiStreamEvent) {
  return typeof event.threadId === "string" ? event.threadId.trim() : "";
}

function eventType(event: ChatAguiStreamEvent) {
  return String(event.type ?? "").trim().toUpperCase();
}

function orderedProtocolEvents(events: readonly ChatAguiStreamEvent[]) {
  const records: OrderedProtocolEvent[] = events.map((event, originalIndex) => ({
    event,
    originalIndex,
    timestamp: typeof event.timestamp === "number" && Number.isFinite(event.timestamp)
      ? event.timestamp
      : null,
  }));
  if (!records.every((record) => record.timestamp !== null)) return records;
  return records.sort((left, right) => (
    (left.timestamp as number) - (right.timestamp as number)
      || left.originalIndex - right.originalIndex
  ));
}

function replayRunLifecycles(
  events: readonly ChatAguiStreamEvent[],
  sessionCode: string,
) {
  const lifecycles = new Map<string, ReplayRunLifecycle>();
  orderedProtocolEvents(events).forEach(({ event }, order) => {
    const threadId = eventThreadId(event);
    if (threadId && threadId !== sessionCode) return;
    const runCode = eventRunCode(event);
    if (!runCode || runCode === "stream") return;
    const type = eventType(event);
    const lifecycle = lifecycles.get(runCode) ?? {
      firstOrder: order,
      runCode,
      startOrder: null,
      supportsSteer: null,
      terminal: false,
    };
    if (type === "RUN_STARTED") {
      lifecycle.startOrder = order;
      lifecycle.terminal = false;
      if (typeof event.supportsSteer === "boolean") {
        lifecycle.supportsSteer = event.supportsSteer;
      }
    } else if (type === "RUN_FINISHED" || type === "RUN_ERROR") {
      lifecycle.terminal = true;
    }
    lifecycles.set(runCode, lifecycle);
  });
  return lifecycles;
}

export function resolveLatestOpenChatReplayRun(
  events: readonly ChatAguiStreamEvent[],
  sessionCode: string,
): ChatReplayRun | null {
  if (!sessionCode) return null;
  const openRuns = Array.from(replayRunLifecycles(events, sessionCode).values())
    .filter((lifecycle) => !lifecycle.terminal)
    .sort((left, right) => (
      (right.startOrder ?? right.firstOrder) - (left.startOrder ?? left.firstOrder)
    ));
  const latest = openRuns[0];
  return latest
    ? { runCode: latest.runCode, supportsSteer: latest.supportsSteer }
    : null;
}

export function isChatReplayRunTerminal(
  events: readonly ChatAguiStreamEvent[],
  sessionCode: string,
  runCode: string,
) {
  if (!sessionCode || !runCode) return false;
  return replayRunLifecycles(events, sessionCode).get(runCode)?.terminal === true;
}
