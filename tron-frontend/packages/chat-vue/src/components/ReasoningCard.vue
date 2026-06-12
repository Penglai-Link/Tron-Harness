<script lang="ts">
import type { AguiUserWorkCard } from "@tron/chat-core";

export type ReasoningValue = Extract<
  AguiUserWorkCard,
  { kind: "reasoning" }
>["value"];
</script>

<script setup lang="ts">
import { BrainCircuit, ChevronDown, ChevronRight } from "lucide-vue-next";
import { computed, ref, watch } from "vue";
import { deriveReasoningPreview } from "@tron/chat-core";
import { cx } from "../utils";
import MarkdownBlock from "./MarkdownBlock.vue";

const props = defineProps<{
  card: ReasoningValue;
}>();

const hasRunningTool = computed(() =>
  props.card.toolCards.some((tc) => tc.started && !tc.ended && !tc.result),
);
const shouldStayExpanded = computed(
  () => !props.card.ended || hasRunningTool.value,
);
const expanded = ref(false);

watch(
  shouldStayExpanded,
  () => {
    expanded.value = shouldStayExpanded.value;
  },
  { immediate: true },
);

const summary = computed(
  () =>
    deriveReasoningPreview(props.card.content) ||
    (props.card.ended ? "Reasoning captured." : "Thinking..."),
);
</script>

<template>
  <section aria-label="Thinking" class="tron-chat__rail-entry">
    <button
      :aria-expanded="expanded"
      class="tron-chat__rail-entry-toggle"
      type="button"
      @click="expanded = !expanded"
    >
      <span class="tron-chat__rail-entry-main">
        <span class="tron-chat__rail-entry-icon">
          <BrainCircuit
            :class="{ 'animate-pulse': !props.card.ended }"
            :size="15"
          />
        </span>
        <span
          :class="
            cx(
              'tron-chat__rail-entry-summary',
              !props.card.ended && 'tron-chat__rail-entry-summary--pending',
            )
          "
        >
          {{ summary }}
        </span>
      </span>
      <span class="tron-chat__rail-entry-chevron">
        <ChevronDown v-if="expanded" :size="14" />
        <ChevronRight v-else :size="14" />
      </span>
    </button>
    <div
      v-if="expanded && props.card.content"
      class="tron-chat__rail-entry-body"
    >
      <div class="tron-chat__rail-detail">
        <MarkdownBlock :content="props.card.content" />
      </div>
    </div>
  </section>
</template>
