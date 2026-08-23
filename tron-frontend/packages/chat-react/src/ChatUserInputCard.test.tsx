// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChatUserInputRequest } from "@tron/chat-core";
import { ChatUserInputCard } from "./ChatUserInputCard";

afterEach(() => cleanup());

const request: ChatUserInputRequest = {
  answers: null,
  assistantMessageCode: "MESSAGE-1",
  intro: "Let's make a few decisions first.",
  questions: [
    {
      options: [
        { description: "First choice", label: "Option one", optionCode: "OPTION-1" },
        { description: "Second choice", label: "Option two", optionCode: "OPTION-2" },
      ],
      question: "Which direction should we take?",
      questionCode: "QUESTION-1",
    },
    {
      options: [
        { description: null, label: "Keep it simple", optionCode: "OPTION-3" },
        { description: null, label: "Make it broad", optionCode: "OPTION-4" },
      ],
      question: "What is the scope?",
      questionCode: "QUESTION-2",
    },
  ],
  requestedAt: Date.now(),
  requestCode: "INPUT-1",
  runCode: "RUN-1",
  sessionCode: "SESSION-1",
  status: "Pending",
};

describe("ChatUserInputCard", () => {
  it("asks one question at a time and preserves answers when going back", () => {
    const onSubmit = vi.fn();
    render(<ChatUserInputCard onSubmit={onSubmit} request={request} />);

    expect(screen.getByText("Question 1 of 2")).toBeTruthy();
    expect(screen.queryByText("What is the scope?")).toBeNull();

    fireEvent.click(screen.getByRole("radio", { name: /Option one/ }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));

    expect(screen.getByText("Question 2 of 2")).toBeTruthy();
    expect(screen.getByText(/What is the scope\?/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Back" }) as HTMLButtonElement).disabled).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect((screen.getByRole("radio", { name: /Option one/ }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("button", { name: "Next" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("submits all answers only after the last question", () => {
    const onSubmit = vi.fn();
    render(<ChatUserInputCard onSubmit={onSubmit} request={request} />);

    fireEvent.click(screen.getByRole("radio", { name: /Option two/ }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(onSubmit).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("radio", { name: /Keep it simple/ }));
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(onSubmit).toHaveBeenCalledWith([
      {
        answerKind: "Option",
        otherText: null,
        optionCode: "OPTION-2",
        questionCode: "QUESTION-1",
      },
      {
        answerKind: "Option",
        otherText: null,
        optionCode: "OPTION-3",
        questionCode: "QUESTION-2",
      },
    ]);
  });
});
