import { useMemo, useState } from "react";
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
  const [selected, setSelected] = useState<Record<string, string>>({});
  const [otherText, setOtherText] = useState<Record<string, string>>({});
  const complete = request.questions.every((question) => {
    const optionCode = selected[question.questionCode] ?? "";
    if (!optionCode) return false;
    if (optionCode.startsWith("OTHER-")) {
      return Boolean(otherText[question.questionCode]?.trim());
    }
    return true;
  });
  const answerCount = useMemo(() => request.questions.length, [request.questions.length]);

  function submit() {
    if (disabled || !complete) return;
    onSubmit(request.questions.map((question) => {
      const optionCode = selected[question.questionCode];
      if (optionCode.startsWith("OTHER-")) {
        return {
          questionCode: question.questionCode,
          answerKind: "Other",
          optionCode: null,
          otherText: otherText[question.questionCode]?.trim() ?? "",
        };
      }
      if (optionCode.startsWith("SKIP-")) {
        return {
          questionCode: question.questionCode,
          answerKind: "Skipped",
          optionCode: null,
          otherText: null,
        };
      }
      return {
        questionCode: question.questionCode,
        answerKind: "Option",
        optionCode,
        otherText: null,
      };
    }));
  }

  return (
    <section className="tron-chat__user-input-card" aria-label="Assistant questions">
      <p className="tron-chat__user-input-intro">{request.intro}</p>
      {request.questions.map((question, index) => {
        const selectedCode = selected[question.questionCode] ?? "";
        const otherSelected = selectedCode.startsWith("OTHER-");
        return (
          <fieldset className="tron-chat__user-input-question" key={question.questionCode}>
            <legend>{index + 1}. {question.question}</legend>
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
                aria-label={`Other answer for question ${index + 1}`}
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
        );
      })}
      <button
        className="tron-chat__user-input-submit"
        disabled={disabled || !complete}
        type="button"
        onClick={submit}
      >
        {disabled ? "Continuing…" : `Continue (${answerCount})`}
      </button>
    </section>
  );
}
