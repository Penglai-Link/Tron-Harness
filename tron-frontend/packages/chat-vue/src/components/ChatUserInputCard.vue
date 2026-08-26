<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { ArrowRight, ChevronLeft, ChevronRight, PencilLine } from "lucide-vue-next";
import type { ChatUserInputAnswer, ChatUserInputLabels, ChatUserInputRequest } from "@tron/chat-core";

const defaultLabels: ChatUserInputLabels = {
  ariaLabel: "Assistant questions",
  progress: (current, total) => `Question ${current} of ${total}`,
  back: "Back",
  next: "Next",
  continue: "Continue",
  confirm: "Confirm",
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
const currentOtherText = computed(() => currentQuestion.value ? otherText[currentQuestion.value.questionCode]?.trim() ?? "" : "");
const complete = computed(() => Boolean(selectedCode.value) || Boolean(currentOtherText.value));
const hasAnswer = computed(() => Boolean(currentOtherText.value) || Boolean(selectedCode.value && !selectedCode.value.startsWith("SKIP-")));
const isLastQuestion = computed(() => currentIndex.value === props.request.questions.length - 1);

watch(() => props.request.requestCode, () => {
  currentIndex.value = 0;
  Object.keys(selected).forEach((key) => delete selected[key]);
  Object.keys(otherText).forEach((key) => delete otherText[key]);
});

function answerFor(questionCode: string): ChatUserInputAnswer {
  const optionCode = selected[questionCode] ?? "";
  const customText = otherText[questionCode]?.trim() ?? "";
  if (customText || optionCode.startsWith("OTHER-")) return { questionCode, answerKind: "Other", optionCode: null, otherText: customText };
  if (optionCode.startsWith("SKIP-")) return { questionCode, answerKind: "Skipped", optionCode: null, otherText: null };
  return { questionCode, answerKind: "Option", optionCode, otherText: null };
}

function goNext() {
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

function skipQuestion() {
  if (props.disabled || !currentQuestion.value) return;
  const question = currentQuestion.value;
  const skippedAnswer: ChatUserInputAnswer = {
    questionCode: question.questionCode,
    answerKind: "Skipped",
    optionCode: null,
    otherText: null,
  };
  selected[question.questionCode] = `SKIP-${question.questionCode}`;
  otherText[question.questionCode] = "";
  if (!isLastQuestion.value) {
    currentIndex.value += 1;
    return;
  }
  emit("submit", props.request.questions.map((item) => item.questionCode === question.questionCode ? skippedAnswer : answerFor(item.questionCode)));
}

function customOptionCode() {
  return currentQuestion.value?.options.find((option) => option.optionCode.startsWith("OTHER-"))?.optionCode
    ?? `OTHER-${currentQuestion.value?.questionCode ?? "ANSWER"}`;
}

function updateOtherText(value: string) {
  if (!currentQuestion.value) return;
  const questionCode = currentQuestion.value.questionCode;
  otherText[questionCode] = value;
  selected[questionCode] = value.trim() ? customOptionCode() : selected[questionCode] ?? "";
}

function handleOtherInput(event: Event) {
  updateOtherText((event.target as HTMLTextAreaElement).value);
}

function selectOption(optionCode: string) {
  if (!currentQuestion.value) return;
  selected[currentQuestion.value.questionCode] = optionCode;
  otherText[currentQuestion.value.questionCode] = "";
}

function handleOtherKeydown(event: KeyboardEvent) {
  if (event.key !== "Enter" || event.shiftKey || event.isComposing) return;
  event.preventDefault();
  goNext();
}

function submitOtherOrSkip() {
  if (hasAnswer.value) {
    goNext();
    return;
  }
  skipQuestion();
}

function isOtherOption(optionCode: string) {
  return optionCode.startsWith("OTHER-");
}

function isSkipOption(optionCode: string) {
  return optionCode.startsWith("SKIP-");
}
</script>

<template>
  <section v-if="currentQuestion" class="tron-chat__user-input-card" :aria-label="labels.ariaLabel">
    <div class="tron-chat__user-input-body">
      <div class="tron-chat__user-input-header">
        <p class="tron-chat__user-input-title">{{ currentQuestion.question }}</p>
        <div class="tron-chat__user-input-navigation" :aria-label="labels.progress(currentIndex + 1, request.questions.length)">
          <button class="tron-chat__user-input-nav-button" :aria-label="labels.back" :disabled="disabled || currentIndex === 0" type="button" @click="goBack">
            <ChevronLeft :size="15" aria-hidden="true" />
          </button>
          <span aria-live="polite">{{ labels.progress(currentIndex + 1, request.questions.length) }}</span>
          <button class="tron-chat__user-input-nav-button" :aria-label="isLastQuestion ? labels.continue : labels.next" :disabled="disabled || !complete" type="button" @click="goNext">
            <ChevronRight :size="15" aria-hidden="true" />
          </button>
        </div>
      </div>
      <fieldset :key="currentQuestion.questionCode" class="tron-chat__user-input-question">
        <legend>{{ currentQuestion.question }}</legend>
        <div class="tron-chat__user-input-options">
          <label v-for="(option, optionIndex) in currentQuestion.options.filter((item) => !isOtherOption(item.optionCode) && !isSkipOption(item.optionCode))" :key="option.optionCode" :class="['tron-chat__user-input-option', { 'is-selected': selectedCode === option.optionCode }]">
            <span class="tron-chat__user-input-option-index" aria-hidden="true">{{ optionIndex + 1 }}</span>
            <input class="tron-chat__user-input-option-radio" :checked="selectedCode === option.optionCode" :disabled="disabled" :name="currentQuestion.questionCode" type="radio" :value="option.optionCode" @change="selectOption(option.optionCode)" />
            <span class="tron-chat__user-input-option-content">
              <strong>{{ option.label }}</strong>
              <small v-if="option.description">{{ option.description }}</small>
            </span>
            <ArrowRight v-if="selectedCode === option.optionCode" class="tron-chat__user-input-option-arrow" :size="15" aria-hidden="true" />
          </label>
        </div>
      </fieldset>
    </div>
    <div class="tron-chat__user-input-actions">
      <label class="tron-chat__user-input-other">
        <PencilLine :size="15" aria-hidden="true" />
        <textarea class="tron-chat__user-input-other-answer" :disabled="disabled" :aria-label="labels.otherAnswerAriaLabel(currentIndex + 1)" maxlength="2000" :placeholder="labels.otherAnswerPlaceholder" rows="1" :value="otherText[currentQuestion.questionCode] ?? ''" @input="handleOtherInput" @keydown="handleOtherKeydown" />
      </label>
      <button :class="['tron-chat__user-input-skip', { 'is-confirm': hasAnswer }]" :disabled="disabled" type="button" @click="submitOtherOrSkip">{{ hasAnswer ? labels.confirm ?? labels.continue : labels.skipOptionLabel }}</button>
    </div>
  </section>
</template>
