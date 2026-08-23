import { useEffect, useState } from "react";
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
  useEffect(() => {
    setCurrentIndex(0);
    setSelected({});
    setOtherText({});
  }, [request.requestCode]);

  const question = request.questions[currentIndex];
  const selectedCode = question ? selected[question.questionCode] ?? "" : "";
  const otherSelected = selectedCode.startsWith("OTHER-");
  const currentComplete = Boolean(selectedCode)
    && (!otherSelected || Boolean(otherText[question?.questionCode ?? ""]?.trim()));
  const isLastQuestion = currentIndex === request.questions.length - 1;

  function answerFor(questionCode: string): ChatUserInputAnswer {
    const optionCode = selected[questionCode] ?? "";
    if (optionCode.startsWith("OTHER-")) {
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

  function submit() {
    if (disabled || !currentComplete) return;
    if (!isLastQuestion) {
      setCurrentIndex((index) => index + 1);
      return;
    }
    onSubmit(request.questions.map((item) => answerFor(item.questionCode)));
  }

  function goBack() {
    if (disabled || currentIndex === 0) return;
    setCurrentIndex((index) => index - 1);
  }

  if (!question) return null;

  function optionLabel(option: (typeof question.options)[number]) {
    if (option.optionCode.startsWith("OTHER-")) return labels.otherOptionLabel;
    if (option.optionCode.startsWith("SKIP-")) return labels.skipOptionLabel;
    return option.label;
  }

  function optionDescription(option: (typeof question.options)[number]) {
    if (option.optionCode.startsWith("OTHER-")) return labels.otherOptionDescription;
    if (option.optionCode.startsWith("SKIP-")) return labels.skipOptionDescription;
    return option.description;
  }

  return (
    <section className="tron-chat__user-input-card" aria-label={labels.ariaLabel}>
      <div className="tron-chat__user-input-body">
        <p className="tron-chat__user-input-intro">{request.intro}</p>
        <div className="tron-chat__user-input-progress" aria-live="polite">
          {labels.progress(currentIndex + 1, request.questions.length)}
        </div>
        <fieldset className="tron-chat__user-input-question" key={question.questionCode}>
          <legend>{currentIndex + 1}. {question.question}</legend>
          <div className="tron-chat__user-input-options">
            {question.options.map((option) => (
              <label className="tron-chat__user-input-option" key={option.optionCode}>
                <input
                  checked={selectedCode === option.optionCode}
                  disabled={disabled}
                  name={question.questionCode}
                  type="radio"
                  value={option.optionCode}
                  onChange={() => setSelected((current) => ({ ...current, [question.questionCode]: option.optionCode }))}
                />
                <span>
                  <strong>{optionLabel(option)}</strong>
                  {optionDescription(option) ? <small>{optionDescription(option)}</small> : null}
                </span>
              </label>
            ))}
          </div>
          {otherSelected ? (
            <textarea
              aria-label={labels.otherAnswerAriaLabel(currentIndex + 1)}
              disabled={disabled}
              maxLength={2000}
              placeholder={labels.otherAnswerPlaceholder}
              value={otherText[question.questionCode] ?? ""}
              onChange={(event) => setOtherText((current) => ({
                ...current,
                [question.questionCode]: event.target.value,
              }))}
            />
          ) : null}
        </fieldset>
      </div>
      <div className="tron-chat__user-input-actions">
        <button
          className="tron-chat__user-input-back"
          disabled={disabled || currentIndex === 0}
          type="button"
          onClick={goBack}
        >
          {labels.back}
        </button>
        <button
          className="tron-chat__user-input-submit"
          disabled={disabled || !currentComplete}
          type="button"
          onClick={submit}
        >
          {disabled ? labels.continuing : isLastQuestion ? labels.continue : labels.next}
        </button>
      </div>
    </section>
  );
}
