import { html, nothing } from "lit-html";
import { guard } from "lit-html/directives/guard.js";
import {
  buildAguiUserWorkCards,
  currentAguiWorkSnapshot,
  formatJsonBlockValue,
  type TranscriptAguiEvent,
} from "@tron/chat-core";

import type { AIStreamEvent } from "../../transport/types";
import type { ChatTerminalI18n } from "../i18n";
import { renderMarkdown as renderMarkdownContent } from "../utils/markdown";

export interface AgentActivityState {
  activityEvents: readonly AIStreamEvent[];
  activityStartedAt: number;
  activityNow: number;
  activityComplete: boolean;
  i18n: ChatTerminalI18n;
}

export function renderAgentActivity(state: AgentActivityState) {
  const events = state.activityEvents as readonly TranscriptAguiEvent[];
  const cards = buildAguiUserWorkCards(
    events.filter((event) => event.type.toUpperCase() !== "REASONING_TEXT_CONTENT"),
  );
  const orderedCards = [...cards].sort((left, right) => {
    const leftRank = left.kind === "reasoning" ? 0 : 1;
    const rightRank = right.kind === "reasoning" ? 0 : 1;
    return leftRank - rightRank || left.value.firstMillis - right.value.firstMillis;
  });
  const visibleCards = orderedCards.filter((card) => {
    if (card.kind === "tool") return card.value.parentReasoningId === null;
    if (card.kind === "toolGroup") {
      return card.value.toolCards.every((tool) => tool.parentReasoningId === null);
    }
    return true;
  });
  const snapshot = currentAguiWorkSnapshot([...events]);
  if (!cards.length && !snapshot) return nothing;

  return html`<section class="ai-terminal-model-session__activity" aria-label=${state.i18n.t("activityLabel")}>
    ${snapshot && !cards.length ? renderActivitySnapshot(snapshot, state) : nothing}
    ${visibleCards.map((card) => renderActivityCard(card, state, events))}
  </section>`;
}

function renderActivitySnapshot(
  snapshot: NonNullable<ReturnType<typeof currentAguiWorkSnapshot>>,
  state: AgentActivityState,
) {
  const { i18n } = state;
  const summary = state.activityComplete
    ? formatElapsedDuration(activityElapsedMilliseconds(state), i18n)
    : snapshot.activity === "THINKING"
      ? i18n.t("reasoningThinking")
      : `${i18n.t("activityProcessed")} ${formatDurationValue(activityElapsedMilliseconds(state), i18n)}`;
  return html`<div class="ai-terminal-model-session__activity-item ai-terminal-model-session__activity-item--snapshot" aria-live="polite">
    ${renderActivityChevron()}
    <span class=${activityStatusClass(state)}>${summary}</span>
  </div>`;
}

function renderActivityCard(
  card: ReturnType<typeof buildAguiUserWorkCards>[number],
  state: AgentActivityState,
  events: readonly TranscriptAguiEvent[],
) {
  const { i18n } = state;
  if (card.kind === "reasoning") {
    const summary = activityStatusLabel(state, events, i18n);
    return html`<details class="ai-terminal-model-session__activity-item">
      <summary>
        ${renderActivityChevron()}
        <span class=${activityStatusClass(state)}>${summary}</span>
      </summary>
      ${card.value.content
        ? html`<div class="ai-terminal-model-session__activity-detail ai-terminal-markdown">
            ${guard([card.value.content], () => renderMarkdownContent(card.value.content))}
          </div>`
        : nothing}
      ${card.value.toolCards.length
        ? renderNestedToolGroup(card.value.toolCards, i18n)
        : nothing}
    </details>`;
  }

  if (card.kind === "toolGroup") {
    const tools = card.value.toolCards.filter((tool) => tool.started);
    const running = tools.some((tool) => !tool.ended && !tool.result);
    return html`<details class="ai-terminal-model-session__activity-item" ?open=${running}>
      <summary>${renderActivityChevron()}<span>${i18n.t("toolsRan")} ${tools.length}</span></summary>
      <div class="ai-terminal-model-session__activity-list">
        ${tools.map((tool) => renderToolActivity(tool, i18n))}
      </div>
    </details>`;
  }

  return renderToolActivity(card.value, i18n);
}

function renderToolActivity(
  tool: Extract<ReturnType<typeof buildAguiUserWorkCards>[number], { kind: "tool" }>["value"],
  i18n: ChatTerminalI18n,
) {
  const complete = tool.ended || Boolean(tool.result);
  const running = tool.started && !complete;
  const args = tool.args === undefined ? "" : formatJsonBlockValue(tool.args);
  const result = tool.result ? formatJsonBlockValue(tool.result) : "";
  const label = tool.toolName || i18n.t("toolUnnamed");

  return html`<details class="ai-terminal-model-session__activity-item ai-terminal-model-session__activity-item--tool" ?open=${running}>
    <summary>${renderActivityChevron()}<span>${running ? i18n.t("toolRunning") : i18n.t("toolRan")} ${label}</span></summary>
    ${args || tool.logs.length || result
      ? html`<div class="ai-terminal-model-session__activity-detail">
          ${args
            ? html`<section><h4>${i18n.t("toolParameters")}</h4><pre>${args}</pre></section>`
            : nothing}
          ${tool.logs.length
            ? html`<section><h4>${i18n.t("toolLog")}</h4><pre>${tool.logs.join("\n")}</pre></section>`
            : nothing}
          ${result
            ? html`<section><h4>${i18n.t("toolResult")}</h4><pre>${result}</pre></section>`
            : nothing}
        </div>`
      : nothing}
  </details>`;
}

function renderNestedToolGroup(
  tools: ReturnType<typeof buildAguiUserWorkCards>[number] extends infer Card
    ? Card extends { kind: "reasoning" }
      ? Card["value"]["toolCards"]
      : never
    : never,
  i18n: ChatTerminalI18n,
) {
  const startedTools = tools.filter((tool) => tool.started);
  if (!startedTools.length) return nothing;
  const running = startedTools.some((tool) => !tool.ended && !tool.result);
  return html`<details class="ai-terminal-model-session__activity-item ai-terminal-model-session__activity-item--nested" ?open=${running}>
    <summary>${renderActivityChevron()}<span>${i18n.t("toolsRan")} ${startedTools.length}</span></summary>
    <div class="ai-terminal-model-session__activity-list">
      ${startedTools.map((tool) => renderToolActivity(tool, i18n))}
    </div>
  </details>`;
}

function renderActivityChevron() {
  return html`<svg
    class="ai-terminal-model-session__activity-chevron"
    xmlns="http://www.w3.org/2000/svg"
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="m9 18 6-6-6-6"></path>
  </svg>`;
}

function activityElapsedMilliseconds(state: AgentActivityState) {
  return Math.max(0, state.activityNow - state.activityStartedAt);
}

function activityStatusLabel(
  state: AgentActivityState,
  events: readonly TranscriptAguiEvent[],
  i18n: ChatTerminalI18n,
) {
  if (state.activityComplete) {
    return formatElapsedDuration(activityElapsedMilliseconds(state), i18n);
  }
  if (hasActiveReasoning(events)) {
    return i18n.t("reasoningThinking");
  }
  return `${i18n.t("activityProcessed")} ${formatDurationValue(activityElapsedMilliseconds(state), i18n)}`;
}

function activityStatusClass(state: AgentActivityState) {
  return state.activityComplete
    ? "ai-terminal-model-session__activity-status"
    : "ai-terminal-model-session__activity-status ai-terminal-model-session__activity-status--running";
}

function hasActiveReasoning(events: readonly TranscriptAguiEvent[]) {
  let active = false;
  events.forEach((event) => {
    const type = event.type.toUpperCase();
    if (type === "REASONING_START" || type === "REASONING_MESSAGE_START") active = true;
    if (type === "REASONING_END" || type === "REASONING_MESSAGE_END") active = false;
  });
  return active;
}

function formatElapsedDuration(milliseconds: number, i18n: ChatTerminalI18n) {
  return `${i18n.t("activityElapsed")} ${formatDurationValue(milliseconds, i18n)}`;
}

function formatDurationValue(milliseconds: number, i18n: ChatTerminalI18n) {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1_000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}${i18n.t("activityMinute")} ${seconds}${i18n.t("activitySecond")}`;
}
