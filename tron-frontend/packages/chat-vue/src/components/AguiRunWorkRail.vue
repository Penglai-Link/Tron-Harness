<script setup lang="ts">
import { ChevronDown, ChevronRight } from "lucide-vue-next";
import { computed, ref, watch } from "vue";
import {
  buildAguiUserWorkCards,
  type TranscriptAguiEvent,
} from "@tron/chat-core";
import ReasoningCard from "./ReasoningCard.vue";
import ToolGroupCard from "./ToolGroupCard.vue";
import ToolCard from "./ToolCard.vue";

const props = withDefaults(defineProps<{
  debugEvents?: string[];
  events?: TranscriptAguiEvent[];
}>(), {
  debugEvents: () => [],
  events: () => [],
});

function formatAguiDuration(milliseconds: number) {
  if (milliseconds < 1000) return "<1s";
  const totalSeconds = Math.max(1, Math.round(milliseconds / 1000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (totalMinutes < 60) return seconds ? `${totalMinutes}m ${seconds}s` : `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`;
}

function aguiWorkDuration(events: TranscriptAguiEvent[]) {
  const times = events
    .map((ev) => ("timestamp" in ev && typeof ev.timestamp === "number" ? ev.timestamp : null))
    .filter((v): v is number => typeof v === "number");
  if (times.length < 2) return "<1s";
  return formatAguiDuration(Math.max(...times) - Math.min(...times));
}

function terminalAguiEvent(events: TranscriptAguiEvent[]) {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const normalized = String(events[i]?.type ?? "").toUpperCase();
    if (normalized.includes("RUN_FINISHED") || normalized.includes("RUN_ERROR")) return events[i];
  }
  return null;
}

const userCards = computed(() => buildAguiUserWorkCards(props.events));
const terminalEvent = computed(() => terminalAguiEvent(props.events));
const expanded = ref(false);
</script>

<template>
  <section
    v-if="userCards.length || props.debugEvents.length"
    aria-label="AG-UI run work"
    class="tron-chat__rail"
  >
    <button
      v-if="terminalEvent"
      :aria-expanded="expanded"
      class="tron-chat__rail-summary"
      type="button"
      @click="expanded = !expanded"
    >
      <span class="tron-chat__rail-summary-label">
        Worked for {{ aguiWorkDuration(props.events) }}
      </span>
      <ChevronDown v-if="expanded" :size="14" />
      <ChevronRight v-else :size="14" />
    </button>
    <div
      v-if="!terminalEvent || expanded"
      class="tron-chat__rail-stack"
      :style="{ marginTop: terminalEvent ? '8px' : '0' }"
    >
      <section
        v-if="userCards.length"
        aria-label="Agent work rail"
        class="tron-chat__rail"
        style="max-width: none; padding-left: 0"
      >
        <div class="tron-chat__rail-card-list">
          <template v-for="card in userCards" :key="card.value.id">
            <ReasoningCard v-if="card.kind === 'reasoning'" :card="card.value" />
            <ToolGroupCard v-else-if="card.kind === 'toolGroup'" :card="card.value" />
            <ToolCard v-else :card="card.value" />
          </template>
        </div>
      </section>
      <section
        v-if="props.debugEvents.length"
        aria-label="AG-UI debug event cards"
        class="tron-chat__rail"
        style="max-width: none; padding-left: 0"
      >
        <div class="tron-chat__debug-list">
          <div
            v-for="(event, index) in props.debugEvents"
            :key="`${event}-${index}`"
            class="tron-chat__debug-item"
          >
            <div class="tron-chat__debug-summary">{{ event }}</div>
          </div>
        </div>
      </section>
    </div>
  </section>
</template>
