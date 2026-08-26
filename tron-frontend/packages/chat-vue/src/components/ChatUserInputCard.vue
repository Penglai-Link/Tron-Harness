<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import type { ChatUserInputAnswer, ChatUserInputLabels, ChatUserInputRequest } from "@tron/chat-core";

const defaultLabels: ChatUserInputLabels = {
  ariaLabel: "Assistant questions",
  progress: (current, total) => `Question ${current} of ${total}`,
  back: "Back",
  next: "Next",
  continue: "Continue",
  continuing: "Continuing…",
  otherOptionLabel: "Other",
  otherOptionDescription: "Provide a custom answer.",
  skipOptionLabel: "Skip",
  skipOptionDescription: "Skip this question.",
  otherAnswerPlaceholder: "Type your answer",
  otherAnswerAriaLabel: (index) => `Other answer for question ${index}`,
};

const props = withDefaults(defineProps<{ request: ChatUserInputRequest; disabled?: boolean; labels?: ChatUserInputLabels }>(), { disabled: false, labels: undefined });
const emit = defineEmits<{ submit: [answers: ChatUserInputAnswer[]] }>();
const currentIndex = ref(0);
const selected = reactive<Record<string, string>>({});
const otherText = reactive<Record<string, string>>({});
const currentQuestion = computed(() => props.request.questions[currentIndex.value]);
const labels = computed(() => props.labels ?? defaultLabels);
const selectedCode = computed(() => currentQuestion.value ? selected[currentQuestion.value.questionCode] ?? "" : "");
const otherSelected = computed(() => selectedCode.value.startsWith("OTHER-"));
const complete = computed(() => Boolean(selectedCode.value)
  && (!otherSelected.value || Boolean(otherText[currentQuestion.value?.questionCode ?? ""]?.trim())));
const isLastQuestion = computed(() => currentIndex.value === props.request.questions.length - 1);

watch(() => props.request.requestCode, () => {
  currentIndex.value = 0;
  Object.keys(selected).forEach((key) => delete selected[key]);
  Object.keys(otherText).forEach((key) => delete otherText[key]);
});

function answerFor(questionCode: string): ChatUserInputAnswer {
  const optionCode = selected[questionCode] ?? "";
  if (optionCode.startsWith("OTHER-")) return { questionCode, answerKind: "Other", optionCode: null, otherText: otherText[questionCode]?.trim() ?? "" };
  if (optionCode.startsWith("SKIP-")) return { questionCode, answerKind: "Skipped", optionCode: null, otherText: null };
  return { questionCode, answerKind: "Option", optionCode, otherText: null };
}

function submit() {
  if (props.disabled || !complete.value) return;
  if (!isLastQuestion.value) {
    currentIndex.value += 1;
    return;
  }
  emit("submit", props.request.questions.map((question) => answerFor(question.questionCode)));
}

function goBack() {
  if (props.disabled || currentIndex.value === 0) return;
  currentIndex.value -= 1;
}

function isOtherOption(optionCode: string) {
  return optionCode.startsWith("OTHER-");
}

function isSkipOption(optionCode: string) {
  return optionCode.startsWith("SKIP-");
}
</script>

<template>
    <section class="tron-chat__user-input-card" :aria-label="labels.ariaLabel">
    <div class="tron-chat__user-input-body">
      <p class="tron-chat__user-input-intro">{{ request.intro }}</p>
      <div class="tron-chat__user-input-progress" aria-live="polite">{{ labels.progress(currentIndex + 1, request.questions.length) }}</div>
      <fieldset v-if="currentQuestion" :key="currentQuestion.questionCode" class="tron-chat__user-input-question">
        <legend>{{ currentIndex + 1 }}. {{ currentQuestion.question }}</legend>
        <div class="tron-chat__user-input-options">
          <label v-for="option in currentQuestion.options" :key="option.optionCode" :class="['tron-chat__user-input-option', { 'is-selected': selectedCode === option.optionCode }]">
            <input v-model="selected[currentQuestion.questionCode]" :disabled="disabled" :name="currentQuestion.questionCode" type="radio" :value="option.optionCode" />
            <span>
              <strong>{{ isOtherOption(option.optionCode) ? labels.otherOptionLabel : isSkipOption(option.optionCode) ? labels.skipOptionLabel : option.label }}</strong>
              <textarea v-if="isOtherOption(option.optionCode) && selectedCode === option.optionCode" v-model="otherText[currentQuestion.questionCode]" class="tron-chat__user-input-other-answer" :disabled="disabled" :aria-label="labels.otherAnswerAriaLabel(currentIndex + 1)" maxlength="2000" :placeholder="labels.otherAnswerPlaceholder" />
              <small v-else-if="isOtherOption(option.optionCode) ? labels.otherOptionDescription : isSkipOption(option.optionCode) ? labels.skipOptionDescription : option.description">{{ isOtherOption(option.optionCode) ? labels.otherOptionDescription : isSkipOption(option.optionCode) ? labels.skipOptionDescription : option.description }}</small>
            </span>
          </label>
        </div>
      </fieldset>
    </div>
    <div class="tron-chat__user-input-actions">
      <button class="tron-chat__user-input-back" :disabled="disabled || currentIndex === 0" type="button" @click="goBack">{{ labels.back }}</button>
      <button class="tron-chat__user-input-submit" :disabled="disabled || !complete" type="button" @click="submit">{{ disabled ? labels.continuing : isLastQuestion ? labels.continue : labels.next }}</button>
    </div>
  </section>
</template>
