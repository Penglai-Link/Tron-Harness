// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import type {
  ChatQueuedSubmission,
  ChatSession,
} from "@tron/chat-core";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TronChatPanel } from "./TronChatPanel";

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: vi.fn(),
    writable: true,
  });
});

afterEach(() => {
  cleanup();
});

function activeSession(): ChatSession {
  return {
    agentCode: "main-agent",
    createdAt: "2026-07-16T00:00:00.000Z",
    id: 1,
    sessionCode: "SESSION-1",
    status: "ACTIVE",
    title: "Panel test",
    updatedAt: "2026-07-16T00:00:00.000Z",
    workspaceCode: "WORKSPACE-1",
  };
}

function renderPanel(
  props: Partial<ComponentProps<typeof TronChatPanel>> = {},
) {
  return render(
    <TronChatPanel
      draft="Follow up"
      messages={[]}
      onDraftChange={() => undefined}
      onSend={() => undefined}
      session={activeSession()}
      showHeader={false}
      {...props}
    />,
  );
}

function queuedSubmission(
  id: string,
  content: string,
): ChatQueuedSubmission {
  return {
    id,
    input: {
      content,
      workspaceCode: "WORKSPACE-1",
    },
    sessionCode: "SESSION-1",
  };
}

describe("TronChatPanel submit controls", () => {
  it("submits the idle draft while preserving Shift+Enter and IME composition", () => {
    const onDraftChange = vi.fn();
    const onSend = vi.fn();
    renderPanel({
      draft: "Follow up",
      onDraftChange,
      onSend,
    });

    const composer = screen.getByRole("textbox");
    composer.setSelectionRange(9, 9);

    fireEvent.keyDown(composer, { key: "Enter", shiftKey: true });
    expect(onDraftChange).toHaveBeenCalledWith("Follow up\n");
    expect(onSend).not.toHaveBeenCalled();

    fireEvent.keyDown(composer, { isComposing: true, key: "Enter" });
    expect(onSend).not.toHaveBeenCalled();

    fireEvent.keyDown(composer, { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(onSend).toHaveBeenNthCalledWith(1, "send");
    expect(onSend).toHaveBeenNthCalledWith(2, "send");
    expect(screen.getByRole("button", { name: "Send" }).title).toBe("Send");
  });

  it("keeps a single Stop action and routes Queue and Steer through shortcuts", () => {
    const onSend = vi.fn();
    const onStop = vi.fn();
    renderPanel({
      onSend,
      onStop,
      runningSubmitModes: ["queue", "steer"],
      sending: true,
      stopLabel: "Stop response",
    });

    const composer = screen.getByRole("textbox");
    const stopButton = screen.getByRole("button", { name: "Stop response" });

    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Steer" })).toBeNull();
    expect(composer.getAttribute("placeholder")).toBe(
      "Enter 加入队列 · ⌘/Ctrl+Enter 立即调整当前回复 · Shift+Enter 换行",
    );
    expect(composer.getAttribute("aria-keyshortcuts")).toBe(
      "Meta+Enter Control+Enter",
    );
    fireEvent.keyDown(composer, { key: "Enter" });
    fireEvent.keyDown(composer, { key: "Enter", metaKey: true });
    fireEvent.keyDown(composer, { ctrlKey: true, key: "Enter" });
    fireEvent.keyDown(composer, { key: "End", metaKey: true });
    fireEvent.click(stopButton);

    expect(onSend).toHaveBeenNthCalledWith(1, "queue");
    expect(onSend).toHaveBeenNthCalledWith(2, "steer");
    expect(onSend).toHaveBeenNthCalledWith(3, "steer");
    expect(onSend).toHaveBeenCalledTimes(3);
    expect(onStop).toHaveBeenCalledTimes(1);
    expect(stopButton.title).toBe("Stop response");
  });

  it("can hide Stop without adding Queue or Steer buttons", () => {
    const onSend = vi.fn();
    renderPanel({
      onSend,
      runningSubmitModes: ["queue", "steer"],
      sending: true,
      showStop: false,
    });

    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Steer" })).toBeNull();
    const composer = screen.getByRole("textbox");
    fireEvent.keyDown(composer, { key: "Enter" });
    fireEvent.keyDown(composer, { key: "Enter", metaKey: true });

    expect(onSend).toHaveBeenNthCalledWith(1, "queue");
    expect(onSend).toHaveBeenNthCalledWith(2, "steer");
  });

  it("uses Enter for Queue and respects its disabled state", () => {
    const onSend = vi.fn();
    const { rerender } = renderPanel({
      onSend,
      onStop: vi.fn(),
      runningSubmitModes: ["queue"],
      sending: true,
    });

    const composer = screen.getByRole("textbox");
    fireEvent.keyDown(composer, { key: "Enter" });
    fireEvent.keyDown(composer, { key: "Enter", metaKey: true });
    expect(onSend).toHaveBeenCalledWith("queue");
    expect(onSend).toHaveBeenCalledTimes(1);
    expect(composer.getAttribute("placeholder")).toBe(
      "Enter 加入队列 · Shift+Enter 换行",
    );
    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Steer" })).toBeNull();

    rerender(
      <TronChatPanel
        draft="Follow up"
        messages={[]}
        onDraftChange={() => undefined}
        onSend={onSend}
        onStop={() => undefined}
        queueDisabled
        runningSubmitModes={["queue"]}
        sending
        session={activeSession()}
        showHeader={false}
      />,
    );

    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" });
    expect(onSend).toHaveBeenCalledTimes(1);
  });

  it("never turns ordinary Enter into Steer when Queue is unavailable", () => {
    const onSend = vi.fn();
    renderPanel({
      onSend,
      runningSubmitModes: ["steer"],
      sending: true,
      showStop: false,
    });

    const composer = screen.getByRole("textbox");
    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Steer" })).toBeNull();
    fireEvent.keyDown(composer, { key: "Enter" });
    fireEvent.keyDown(composer, { key: "Enter", metaKey: true });

    expect(onSend).toHaveBeenCalledTimes(1);
    expect(onSend).toHaveBeenCalledWith("steer");
  });

  it("requires an explicit Stop click when no running submit mode is available", () => {
    const onSend = vi.fn();
    const onStop = vi.fn();
    renderPanel({ onSend, onStop, sending: true });

    expect(screen.queryByRole("button", { name: "Send" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Queue" })).toBeNull();
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));

    expect(onSend).not.toHaveBeenCalled();
    expect(onStop).toHaveBeenCalledTimes(1);
  });

  it("uses the onSend fallback only for an explicit Stop click", () => {
    const onSend = vi.fn();
    renderPanel({ onSend, sending: true });

    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));

    expect(onSend).toHaveBeenCalledTimes(1);
    expect(onSend).toHaveBeenCalledWith("send");
  });

  it("renders the controlled queue and delegates edit and remove actions", () => {
    const onEditQueuedMessage = vi.fn();
    const onRemoveQueuedMessage = vi.fn();
    renderPanel({
      onEditQueuedMessage,
      onRemoveQueuedMessage,
      queuedLabel: "Queued replies",
      queuedMessages: [
        queuedSubmission("QUEUE-1", "First follow-up"),
        queuedSubmission("QUEUE-2", "Second follow-up"),
      ],
    });

    const queue = screen.getByRole("region", { name: "Queued replies" });
    expect(within(queue).getByText("2")).toBeTruthy();
    expect(within(queue).getByText("First follow-up")).toBeTruthy();
    expect(within(queue).getByText("Second follow-up")).toBeTruthy();

    fireEvent.click(
      within(queue).getAllByRole("button", { name: "Edit queued message" })[0],
    );
    fireEvent.click(
      within(queue).getAllByRole("button", { name: "Remove queued message" })[1],
    );

    expect(onEditQueuedMessage).toHaveBeenCalledWith(
      "QUEUE-1",
      "First follow-up",
    );
    expect(onRemoveQueuedMessage).toHaveBeenCalledWith("QUEUE-2");
  });
});
