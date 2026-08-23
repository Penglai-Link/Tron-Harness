import {
  aguiEventMillis,
  aguiEventType,
} from "./transcript";
import type { TranscriptAguiEvent } from "./types";

export function isAguiRunTerminal(events: TranscriptAguiEvent[]) {
  return events.some((event) => {
    const type = aguiEventType(event).toUpperCase();
    return type.includes("RUN_FINISHED") || type.includes("RUN_ERROR");
  });
}

export function aguiWorkElapsedMilliseconds(
  events: TranscriptAguiEvent[],
  now = Date.now(),
) {
  const timestamps = events
    .map(aguiEventMillis)
    .filter((value): value is number => typeof value === "number");
  if (!timestamps.length) return 0;

  const first = Math.min(...timestamps);
  const latest = Math.max(...timestamps);
  const end = isAguiRunTerminal(events) ? latest : Math.max(latest, now);
  return Math.max(0, end - first);
}

export type AguiWorkActivity = "PREPARING" | "THINKING" | "RESPONDING";

export interface AguiWorkSnapshot {
  activity: AguiWorkActivity;
  summary: string | null;
}

export function currentAguiWorkSnapshot(events: TranscriptAguiEvent[]): AguiWorkSnapshot | null {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (!event || aguiEventType(event).toUpperCase() !== "ACTIVITY_SNAPSHOT") continue;
    const activity = String(("activity" in event ? event.activity : "") ?? "").toUpperCase();
    if (activity === "PREPARING" || activity === "THINKING" || activity === "RESPONDING") {
      const summary = "summary" in event && typeof event.summary === "string"
        ? event.summary.trim()
        : "";
      return { activity, summary: summary || null };
    }
  }
  return null;
}

export function currentAguiWorkActivity(events: TranscriptAguiEvent[]) {
  return currentAguiWorkSnapshot(events)?.activity ?? null;
}
