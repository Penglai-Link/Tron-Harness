import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { ArrowRight, ChevronLeft, ChevronRight, PencilLine } from "lucide-react";
import type {
  ChatUserInputAnswer,
  ChatUserInputLabels,
  ChatUserInputRequest,
} from "@tron/chat-core";

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

export function ChatUserInputCard({
  disabled = false,
  labels = defaultLabels,
  onSubmit,
  request,
}: {
  disabled?: boolean;
  labels?: ChatUserInputLabels;
  onSubmit: (answers: ChatUserInputAnswer[]) => void;
  request: ChatUserInputRequest;
}) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [selected, setSelected] = useState<Record<string, string>>({});
  const [otherText, setOtherText] = useState<Record<string, string>>({});
  const cardRef = useRef<HTMLElement>(null);

  useEffect(() => {
    setCurrentIndex(0);
    setSelected({});
    setOtherText({});
  }, [request.requestCode]);

  useEffect(() => {
    const handleWheel = (e: WheelEvent) => {
      e.stopPropagation();
    };

    const card = cardRef.current;
    if (card) {
      card.addEventListener('wheel', handleWheel, { passive: false });
      return () => card.removeEventListener('wheel', handleWheel);
    }
  }, []);

  const question = request.questions[currentIndex];
  const selectedCode = question ? selected[question.questionCode] ?? "" : "";
  const currentOtherText = question ? otherText[question.questionCode]?.trim() ?? "" : "";
  const currentComplete = Boolean(selectedCode) || Boolean(currentOtherText);
  const hasAnswer = Boolean(currentOtherText) || Boolean(selectedCode && !selectedCode.startsWith("SKIP-"));
  const isLastQuestion = currentIndex === request.questions.length - 1;

  function answerFor(questionCode: string): ChatUserInputAnswer {
    const optionCode = selected[questionCode] ?? "";
    const customText = otherText[questionCode]?.trim() ?? "";
    if (customText || optionCode.startsWith("OTHER-")) {
      return {
        questionCode,
        answerKind: "Other",
        optionCode: null,
        otherText: otherText[questionCode]?.trim() ?? "",
      };
    }
    if (optionCode.startsWith("SKIP-")) {
      return {
        questionCode,
        answerKind: "Skipped",
        optionCode: null,
        otherText: null,
      };
    }
    return {
      questionCode,
      answerKind: "Option",
      optionCode,
      otherText: null,
    };
  }

  function goBack() {
    if (disabled || currentIndex === 0) return;
    setCurrentIndex((index) => index - 1);
  }

  function goNext() {
    if (disabled || !currentComplete) return;
    if (!isLastQuestion) {
      setCurrentIndex((index) => index + 1);
      return;
    }
    onSubmit(request.questions.map((item) => answerFor(item.questionCode)));
  }

  function skipQuestion() {
    if (disabled || !question) return;
    const skippedAnswer: ChatUserInputAnswer = {
      questionCode: question.questionCode,
      answerKind: "Skipped",
      optionCode: null,
      otherText: null,
    };
    setSelected((current) => ({ ...current, [question.questionCode]: `SKIP-${question.questionCode}` }));
    setOtherText((current) => ({ ...current, [question.questionCode]: "" }));
    if (!isLastQuestion) {
      setCurrentIndex((index) => index + 1);
      return;
    }
    onSubmit(request.questions.map((item) => (
      item.questionCode === question.questionCode ? skippedAnswer : answerFor(item.questionCode)
    )));
  }

  function customOptionCode() {
    return question?.options.find((option) => option.optionCode.startsWith("OTHER-"))?.optionCode
      ?? `OTHER-${question?.questionCode ?? "ANSWER"}`;
  }

  function updateOtherText(value: string) {
    if (!question) return;
    setOtherText((current) => ({ ...current, [question.questionCode]: value }));
    setSelected((current) => ({
      ...current,
      [question.questionCode]: value.trim() ? customOptionCode() : current[question.questionCode] ?? "",
    }));
  }

  function selectOption(optionCode: string) {
    if (!question) return;
    setSelected((current) => ({ ...current, [question.questionCode]: optionCode }));
    setOtherText((current) => ({ ...current, [question.questionCode]: "" }));
  }

  function handleOtherKeyDown(event: ReactKeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
    event.preventDefault();
    goNext();
  }

  function submitOtherOrSkip() {
    if (hasAnswer) {
      goNext();
      return;
    }
    skipQuestion();
  }

  if (!question) return null;

  return (
    <section className="tron-chat__user-input-card" aria-label={labels.ariaLabel} ref={cardRef}>
      <div className="tron-chat__user-input-body">
        <div className="tron-chat__user-input-header">
          <p className="tron-chat__user-input-title">{question.question}</p>
          <div className="tron-chat__user-input-navigation" aria-label={labels.progress(currentIndex + 1, request.questions.length)}>
            <button
              aria-label={labels.back}
              className="tron-chat__user-input-nav-button"
              disabled={disabled || currentIndex === 0}
              type="button"
              onClick={goBack}
            >
              <ChevronLeft aria-hidden="true" size={15} />
            </button>
            <span aria-live="polite">{labels.progress(currentIndex + 1, request.questions.length)}</span>
            <button
              aria-label={isLastQuestion ? labels.continue : labels.next}
              className="tron-chat__user-input-nav-button"
              disabled={disabled || !currentComplete}
              type="button"
              onClick={goNext}
            >
              <ChevronRight aria-hidden="true" size={15} />
            </button>
          </div>
        </div>
        <fieldset className="tron-chat__user-input-question" key={question.questionCode}>
          <legend>{question.question}</legend>
          <div className="tron-chat__user-input-options">
            {question.options.filter((option) => (
              !option.optionCode.startsWith("OTHER-") && !option.optionCode.startsWith("SKIP-")
            )).map((option, optionIndex) => {
              const isSelected = selectedCode === option.optionCode;
              return (
                <label
                  className={`tron-chat__user-input-option${isSelected ? " is-selected" : ""}`}
                  key={option.optionCode}
                >
                  <span className="tron-chat__user-input-option-index" aria-hidden="true">{optionIndex + 1}</span>
                  <input
                    className="tron-chat__user-input-option-radio"
                    checked={isSelected}
                    disabled={disabled}
                    name={question.questionCode}
                    type="radio"
                    value={option.optionCode}
                    onChange={() => selectOption(option.optionCode)}
                  />
                  <span className="tron-chat__user-input-option-content">
                    <strong>{option.label}</strong>
                    {option.description ? <small>{option.description}</small> : null}
                  </span>
                  {isSelected ? <ArrowRight aria-hidden="true" className="tron-chat__user-input-option-arrow" size={15} /> : null}
                </label>
              );
            })}
          </div>
        </fieldset>
      </div>
      <div className="tron-chat__user-input-actions">
        <label className="tron-chat__user-input-other">
          <PencilLine aria-hidden="true" size={15} />
          <textarea
            aria-label={labels.otherAnswerAriaLabel(currentIndex + 1)}
            className="tron-chat__user-input-other-answer"
            disabled={disabled}
            maxLength={2000}
            placeholder={labels.otherAnswerPlaceholder}
            rows={1}
            value={otherText[question.questionCode] ?? ""}
            onChange={(event) => updateOtherText(event.target.value)}
            onKeyDown={handleOtherKeyDown}
          />
        </label>
        <button
          className={`tron-chat__user-input-skip${hasAnswer ? " is-confirm" : ""}`}
          disabled={disabled}
          type="button"
          onClick={submitOtherOrSkip}
        >
          {hasAnswer ? labels.confirm ?? labels.continue : labels.skipOptionLabel}
        </button>
      </div>
    </section>
  );
}
