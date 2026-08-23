import { useEffect, useState } from "react";
import type {
  ChatUserInputAnswer,
  ChatUserInputRequest,
} from "@tron/chat-core";

export function ChatUserInputCard({
  disabled = false,
  onSubmit,
  request,
}: {
  disabled?: boolean;
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

  return (
    <section className="tron-chat__user-input-card" aria-label="Assistant questions">
      <p className="tron-chat__user-input-intro">{request.intro}</p>
      <div className="tron-chat__user-input-progress" aria-live="polite">
        Question {currentIndex + 1} of {request.questions.length}
      </div>
      <fieldset className="tron-chat__user-input-question">
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
                <strong>{option.label}</strong>
                {option.description ? <small>{option.description}</small> : null}
              </span>
            </label>
          ))}
        </div>
        {otherSelected ? (
          <textarea
            aria-label={`Other answer for question ${currentIndex + 1}`}
            disabled={disabled}
            maxLength={2000}
            placeholder="Type your answer"
            value={otherText[question.questionCode] ?? ""}
            onChange={(event) => setOtherText((current) => ({
              ...current,
              [question.questionCode]: event.target.value,
            }))}
          />
        ) : null}
      </fieldset>
      <div className="tron-chat__user-input-actions">
        <button
          className="tron-chat__user-input-back"
          disabled={disabled || currentIndex === 0}
          type="button"
          onClick={goBack}
        >
          Back
        </button>
        <button
          className="tron-chat__user-input-submit"
          disabled={disabled || !currentComplete}
          type="button"
          onClick={submit}
        >
          {disabled ? "Continuing…" : isLastQuestion ? "Continue" : "Next"}
        </button>
      </div>
    </section>
  );
}
