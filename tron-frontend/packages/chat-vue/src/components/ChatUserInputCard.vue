<script setup lang="ts">
import { computed, reactive } from "vue";
import type { ChatUserInputAnswer, ChatUserInputRequest } from "@tron/chat-core";

const props = withDefaults(defineProps<{ request: ChatUserInputRequest; disabled?: boolean }>(), { disabled: false });
const emit = defineEmits<{ submit: [answers: ChatUserInputAnswer[]] }>();
const selected = reactive<Record<string, string>>({});
const otherText = reactive<Record<string, string>>({});
const complete = computed(() => props.request.questions.every((question) => {
  const optionCode = selected[question.questionCode] ?? "";
  if (!optionCode) return false;
  if (optionCode.startsWith("OTHER-")) return Boolean(otherText[question.questionCode]?.trim());
  return true;
}));

function submit() {
  if (props.disabled || !complete.value) return;
  emit("submit", props.request.questions.map((question) => {
    const optionCode = selected[question.questionCode];
    if (optionCode.startsWith("OTHER-")) return { questionCode: question.questionCode, answerKind: "Other", optionCode: null, otherText: otherText[question.questionCode]?.trim() ?? "" };
    if (optionCode.startsWith("SKIP-")) return { questionCode: question.questionCode, answerKind: "Skipped", optionCode: null, otherText: null };
    return { questionCode: question.questionCode, answerKind: "Option", optionCode, otherText: null };
  }));
}
</script>

<template>
  <section class="tron-chat__user-input-card" aria-label="Assistant questions">
    <p class="tron-chat__user-input-intro">{{ request.intro }}</p>
    <fieldset v-for="(question, index) in request.questions" :key="question.questionCode" class="tron-chat__user-input-question">
      <legend>{{ index + 1 }}. {{ question.question }}</legend>
      <div class="tron-chat__user-input-options">
        <label v-for="option in question.options" :key="option.optionCode" class="tron-chat__user-input-option">
          <input v-model="selected[question.questionCode]" :disabled="disabled" :name="question.questionCode" type="radio" :value="option.optionCode" />
          <span><strong>{{ option.label }}</strong><small v-if="option.description">{{ option.description }}</small></span>
        </label>
      </div>
      <textarea v-if="selected[question.questionCode]?.startsWith('OTHER-')" v-model="otherText[question.questionCode]" :disabled="disabled" :aria-label="`Other answer for question ${index + 1}`" maxlength="2000" placeholder="Type your answer" />
    </fieldset>
    <button class="tron-chat__user-input-submit" :disabled="disabled || !complete" type="button" @click="submit">{{ disabled ? "Continuing…" : `Continue (${request.questions.length})` }}</button>
  </section>
</template>
