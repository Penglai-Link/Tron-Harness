import { deriveReasoningPreview } from "./display";
import {
  aguiEventMillis,
  aguiEventType,
  buildAguiUserWorkCards,
  toolSummaryLabel,
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

export function currentAguiWorkSummary(events: TranscriptAguiEvent[]) {
  const cards = buildAguiUserWorkCards(events);
  const reasoningCards = cards.filter((card) => card.kind === "reasoning");
  const latestReasoning = reasoningCards.at(-1)?.value;
  if (latestReasoning && !latestReasoning.ended) {
    return deriveReasoningPreview(latestReasoning.content);
  }

  for (let index = reasoningCards.length - 1; index >= 0; index -= 1) {
    const summary = deriveReasoningPreview(reasoningCards[index]?.value.content ?? "");
    if (summary) return summary;
  }

  const tools = cards.flatMap((card) => (
    card.kind === "reasoning"
      ? card.value.toolCards
      : card.kind === "toolGroup"
        ? card.value.toolCards
        : [card.value]
  ));
  const latestTool = tools.reduce<typeof tools[number] | null>(
    (latest, tool) => (!latest || tool.firstMillis > latest.firstMillis ? tool : latest),
    null,
  );
  return latestTool ? toolSummaryLabel(latestTool.toolName, latestTool.args) : "";
}
