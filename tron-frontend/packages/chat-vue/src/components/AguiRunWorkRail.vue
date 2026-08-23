<script setup lang="ts">
import { BrainCircuit, ChevronDown, ChevronRight } from "lucide-vue-next";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  aguiWorkElapsedMilliseconds,
  aguiReasoningEffort,
  buildAguiUserWorkCards,
  currentAguiWorkActivity,
  isAguiRunTerminal,
  type TranscriptAguiEvent,
  type ChatWorkRailLabels,
} from "@tron/chat-core";
import ReasoningCard from "./ReasoningCard.vue";
import ToolGroupCard from "./ToolGroupCard.vue";
import ToolCard from "./ToolCard.vue";

const props = withDefaults(defineProps<{
  debugEvents?: string[];
  events?: TranscriptAguiEvent[];
  labels?: ChatWorkRailLabels;
}>(), {
  debugEvents: () => [],
  events: () => [],
  labels: undefined,
});

const defaultLabels: ChatWorkRailLabels = {
  ariaLabel: "Agent work",
  preparing: "Preparing request…",
  responding: "Drafting response…",
  thinking: "Thinking…",
  workedFor: (duration) => `Worked for ${duration}`,
  workingFor: (duration) => `Working for ${duration}`,
};

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

const userCards = computed(() => buildAguiUserWorkCards(props.events));
const reasoningEffort = computed(() => aguiReasoningEffort(props.events));
const terminalEvent = computed(() => isAguiRunTerminal(props.events));
const expanded = ref(false);
const now = ref(Date.now());
const labels = computed(() => props.labels ?? defaultLabels);
const duration = computed(() => formatAguiDuration(aguiWorkElapsedMilliseconds(props.events, now.value)));
const currentSummary = computed(() => {
  const activity = currentAguiWorkActivity(props.events);
  if (activity === "PREPARING") return labels.value.preparing;
  if (activity === "RESPONDING") return labels.value.responding;
  return labels.value.thinking;
});
let timer: ReturnType<typeof setInterval> | undefined;

watch(terminalEvent, (completed) => {
  if (timer) clearInterval(timer);
  if (completed) {
    expanded.value = false;
    return;
  }
  now.value = Date.now();
  timer = setInterval(() => {
    now.value = Date.now();
  }, 1_000);
}, { immediate: true });

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <section
    v-if="props.events.length || userCards.length || props.debugEvents.length"
    :aria-label="labels.ariaLabel"
    class="tron-chat__rail"
  >
    <button
      :aria-expanded="expanded"
      class="tron-chat__rail-summary"
      type="button"
      @click="expanded = !expanded"
    >
      <span class="tron-chat__rail-summary-label">
        {{ terminalEvent ? labels.workedFor(duration) : labels.workingFor(duration) }}
        <span v-if="reasoningEffort"> · {{ reasoningEffort }}</span>
      </span>
      <ChevronDown v-if="expanded" :size="14" />
      <ChevronRight v-else :size="14" />
    </button>
    <div v-if="!terminalEvent" aria-live="polite" class="tron-chat__rail-current">
      <BrainCircuit class="tron-chat__rail-current-icon animate-pulse" :size="15" />
      <span class="tron-chat__rail-current-text">{{ currentSummary }}</span>
    </div>
    <div
      v-if="expanded"
      class="tron-chat__rail-stack"
      :style="{ marginTop: '8px' }"
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
