// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import type {
  ChatAguiStreamEvent,
  ChatMessage,
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

function message(role: ChatMessage["role"], content: string): ChatMessage {
  return {
    actor: role === "USER" ? "Operator" : "Assistant",
    channel: "MAIN_AGENT",
    content,
    createdAt: "2026-07-16T10:59:00.000Z",
    id: `${role}-${content}`,
    messageCode: `${role}-${content}`,
    role,
    sessionCode: "SESSION-1",
  };
}

describe("TronChatPanel submit controls", () => {
  it("shows a stable activity stage while running and auto-collapses when complete", () => {
    const startedAt = Date.now() - 2_000;
    const messages = [{
      actor: "User",
      channel: "MAIN_AGENT",
      content: "Inspect the implementation",
      createdAt: new Date(startedAt).toISOString(),
      id: 1,
      messageCode: "MSG-USER",
      role: "USER" as const,
      sessionCode: "SESSION-1",
    }];
    const activeEvents: ChatAguiStreamEvent[] = [
      { runId: "RUN-1", threadId: "SESSION-1", timestamp: startedAt, type: "RUN_STARTED" },
      { activity: "THINKING", runId: "RUN-1", summary: "Inspecting the implementation plan.", threadId: "SESSION-1", timestamp: startedAt + 1, type: "ACTIVITY_SNAPSHOT" },
      { messageId: "reasoning-1", runId: "RUN-1", threadId: "SESSION-1", timestamp: startedAt + 2, type: "REASONING_MESSAGE_START" },
      { delta: "Inspecting the implementation.", messageId: "reasoning-1", runId: "RUN-1", threadId: "SESSION-1", timestamp: startedAt + 3, type: "REASONING_MESSAGE_CONTENT" },
    ];
    const panelProps = {
      draft: "Follow up",
      messages,
      onDraftChange: () => undefined,
      onSend: () => undefined,
      session: activeSession(),
      showHeader: false,
    };
    const { rerender } = render(<TronChatPanel {...panelProps} protocolEvents={activeEvents} />);

    const summaryButton = screen.getByRole("button", { name: /Working for/ });
    expect(screen.getByText("Inspecting the implementation plan.")).toBeTruthy();
    fireEvent.click(summaryButton);
    expect(summaryButton.getAttribute("aria-expanded")).toBe("true");

    rerender(
      <TronChatPanel
        {...panelProps}
        protocolEvents={[
          ...activeEvents,
          { runId: "RUN-1", threadId: "SESSION-1", timestamp: startedAt + 3_000, type: "RUN_FINISHED" },
        ]}
      />,
    );

    const completedButton = screen.getByRole("button", { name: /Worked for/ });
    expect(completedButton.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText("Inspecting the implementation plan.")).toBeNull();
    fireEvent.click(completedButton);
    expect(completedButton.getAttribute("aria-expanded")).toBe("true");
  });

  it("only labels an active empty assistant message as waiting", () => {
    const userMessage = {
      actor: "User",
      channel: "MAIN_AGENT",
      content: "Create the document",
      createdAt: "2026-07-16T00:00:00.000Z",
      id: 1,
      messageCode: "MSG-USER",
      role: "USER" as const,
      sessionCode: "SESSION-1",
    };
    const emptyAssistant = {
      actor: "Assistant",
      channel: "MAIN_AGENT",
      content: "",
      createdAt: "2026-07-16T00:00:00.000Z",
      id: 2,
      messageCode: "MSG-ASSISTANT",
      role: "ASSISTANT" as const,
      sessionCode: "SESSION-1",
    };
    const messages = [userMessage, emptyAssistant];
    const { rerender } = renderPanel({ messages });

    expect(screen.getByText("No assistant response was produced.")).toBeTruthy();
    expect(screen.queryByText("Waiting for assistant output...")).toBeNull();

    rerender(
      <TronChatPanel
        draft="Follow up"
        messages={[
          ...messages,
          {
            ...userMessage,
            content: "Try again",
            id: 3,
            messageCode: "MSG-USER-RETRY",
          },
        ]}
        onDraftChange={() => undefined}
        onSend={() => undefined}
        sending
        session={activeSession()}
        showHeader={false}
      />,
    );

    expect(screen.getByText("No assistant response was produced.")).toBeTruthy();
    expect(screen.queryByText("Waiting for assistant output...")).toBeNull();

    rerender(
      <TronChatPanel
        draft="Follow up"
        messages={messages}
        onDraftChange={() => undefined}
        onSend={() => undefined}
        sending
        session={activeSession()}
        showHeader={false}
      />,
    );

    expect(screen.getByText("Waiting for assistant output...")).toBeTruthy();
    expect(screen.queryByText("No assistant response was produced.")).toBeNull();
  });

  it("renders message actions below the message content", () => {
    renderPanel({
      messages: [
        message("USER", "User message"),
        message("ASSISTANT", "Assistant message"),
      ],
      onEditMessage: vi.fn(),
    });

    const userMessage = screen.getByText("User message", {
      selector: ".tron-chat__message-body",
    }).closest("article");
    const assistantMessage = screen.getByText("Assistant message", {
      selector: ".markdown-body p",
    }).closest("article");

    expect(userMessage?.querySelector(".tron-chat__message-header .tron-chat__message-actions")).toBeNull();
    expect(assistantMessage?.querySelector(".tron-chat__message-header .tron-chat__message-actions")).toBeNull();
    expect(userMessage?.querySelector(".tron-chat__message-actions")).toBeNull();
    expect(assistantMessage?.querySelector(".tron-chat__message-actions")).toBeNull();
    expect(userMessage?.nextElementSibling?.classList.contains("tron-chat__message-actions--outside")).toBe(true);
    expect(assistantMessage?.nextElementSibling?.classList.contains("tron-chat__message-actions--outside")).toBe(true);
  });

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

  it("uploads an image pasted into the composer instead of inserting it as text", () => {
    const onDraftChange = vi.fn();
    const onPasteAttachment = vi.fn();
    renderPanel({ onDraftChange, onPasteAttachment });

    const file = new File(["image"], "", { type: "image/png" });
    fireEvent.paste(screen.getByRole("textbox"), {
      clipboardData: {
        items: [{
          getAsFile: () => file,
          kind: "file",
          type: "image/png",
        }],
      },
    });

    expect(onDraftChange).not.toHaveBeenCalled();
    expect(onPasteAttachment).toHaveBeenCalledTimes(1);
    expect(onPasteAttachment.mock.calls[0][0]).toMatchObject({
      name: expect.stringMatching(/^pasted-image-\d+\.png$/),
      type: "image/png",
    });
  });

  it("opens a preview when a historical image attachment is clicked", () => {
    renderPanel({
      messages: [{
        ...message("USER", "What is this image?"),
        attachments: [{
          contentType: "image/png",
          fileName: "screenshot.png",
          id: "IMAGE-1",
          imageUrl: "data:image/png;base64,AA==",
          kind: "image",
          metadata: {},
          summary: "screenshot.png",
        }],
      }],
    });

    fireEvent.click(screen.getByRole("button", { name: "Preview screenshot.png" }));

    const dialog = screen.getByRole("dialog", { name: "Image preview" });
    expect(within(dialog).getByRole("img", { name: "screenshot.png" })).toBeTruthy();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Image preview" })).toBeNull();
  });

  it("shows annotation content while its tag is hovered and supports translated labels", () => {
    renderPanel({
      attachmentLabels: {
        annotationPreview: "注释预览",
        attachments: "附件",
      },
      formatAnnotationCount: (count) => `${count} 条注释`,
      messages: [{
        ...message("USER", "Inspect this element"),
        attachments: [{
          id: "INSPECTION-1",
          imageUrl: "data:image/png;base64,AA==",
          kind: "inspection",
          metadata: {},
          summary: "页面按钮",
        }],
      }],
    });

    const tag = screen.getByRole("button", { name: "注释预览: 1 条注释" });
    expect(screen.queryByRole("tooltip", { name: "注释预览" })).toBeNull();
    fireEvent.mouseEnter(tag);

    const preview = screen.getByRole("tooltip", { name: "注释预览" });
    expect(preview.textContent).toContain("页面按钮");
    expect(within(preview).getByRole("img", { name: "页面按钮" })).toBeTruthy();

    fireEvent.mouseLeave(tag.parentElement!);
    expect(screen.queryByRole("tooltip", { name: "注释预览" })).toBeNull();
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
