import type {
  ChatAguiEvent,
  ChatMessage,
  ChatSendResult,
  ChatSession,
  ChatSessionGroup,
  CreateChatSessionInput,
  SendChatMessageInput,
  UpdateChatSessionInput,
} from "@/domain/chat/model/ChatModel";

const baseTime = "2026-05-27T09:30:00Z";

const sessions: ChatSession[] = [
  {
    id: 735200000000000001,
    sessionCode: "SESSION-8842",
    workspaceCode: "TRON_CORE",
    userCode: "USER-AUTH",
    appCode: "TRON",
    taskCode: "TASK-8842",
    title: "Primary investigation",
    status: "ACTIVE",
    lastRunCode: "RUN-42",
    lastRunStatus: "COMPLETED",
    createdAt: baseTime,
    updatedAt: "2026-05-27T10:44:00Z",
  },
  {
    id: 735200000000000002,
    sessionCode: "SESSION-REFACTOR",
    workspaceCode: "TRON_CORE",
    userCode: "USER-AUTH",
    appCode: "TRON",
    taskCode: "TASK-8842",
    title: "Refactor discussion",
    status: "ACTIVE",
    lastRunCode: "RUN-39",
    lastRunStatus: "COMPLETED",
    createdAt: baseTime,
    updatedAt: "2026-05-27T09:50:00Z",
  },
  {
    id: 735200000000000003,
    sessionCode: "SESSION-UNASSIGNED",
    workspaceCode: "TRON_CORE",
    userCode: "USER-AUTH",
    appCode: null,
    taskCode: null,
    title: "Historical trace review",
    status: "ACTIVE",
    lastRunCode: null,
    lastRunStatus: null,
    createdAt: baseTime,
    updatedAt: "2026-05-27T08:20:00Z",
  },
];

const messagesBySession: Record<string, ChatMessage[]> = {
  "SESSION-8842": [
    {
      id: 1,
      messageCode: "MSG-1",
      sessionCode: "SESSION-8842",
      runCode: "RUN-42",
      role: "USER",
      channel: "TASK_AGENT",
      actor: "User",
      content: "Auth latency spiked after the token validator deployment. Find the narrowest rollback path.",
      createdAt: "2026-05-27T10:42:00Z",
    },
    {
      id: 2,
      messageCode: "MSG-2",
      sessionCode: "SESSION-8842",
      runCode: "RUN-42",
      role: "ASSISTANT",
      channel: "TASK_AGENT",
      actor: "Assistant",
      content: "The narrowest rollback is to restore the fast-path cache check behind a feature flag, then watch eu-west-1 and eu-central-1 latency before widening rollout.",
      createdAt: "2026-05-27T10:44:00Z",
    },
  ],
  "SESSION-REFACTOR": [
    {
      id: 3,
      messageCode: "MSG-3",
      sessionCode: "SESSION-REFACTOR",
      runCode: "RUN-39",
      role: "USER",
      channel: "TASK_AGENT",
      actor: "User",
      content: "Keep the refactor focused on the connector boundary.",
      createdAt: "2026-05-27T09:47:00Z",
    },
    {
      id: 4,
      messageCode: "MSG-4",
      sessionCode: "SESSION-REFACTOR",
      runCode: "RUN-39",
      role: "ASSISTANT",
      channel: "TASK_AGENT",
      actor: "Assistant",
      content: "I will keep persistence and runtime dispatch untouched, and only move connector mapping behind the existing adapter seam.",
      createdAt: "2026-05-27T09:50:00Z",
    },
  ],
  "SESSION-UNASSIGNED": [],
};

export function getChatSessionGroups(): ChatSessionGroup[] {
  return [
    {
      taskCode: "TASK-8842",
      taskTitle: "TASK-8842",
      sessions: sessions.filter((session) => session.taskCode === "TASK-8842"),
    },
    {
      taskCode: null,
      taskTitle: "Chat",
      sessions: sessions.filter((session) => !session.taskCode),
    },
  ];
}

export function getChatMessages(sessionCode: string): ChatMessage[] {
  return messagesBySession[sessionCode] ?? [];
}

export function getChatAguiEvents(sessionCode: string): ChatAguiEvent[] {
  const session = sessions.find((item) => item.sessionCode === sessionCode);
  if (!session?.lastRunCode) return [];
  return [
    {
      id: "EVT-1",
      type: "agent.run.started",
      eventName: "agent.run.started",
      runCode: session.lastRunCode,
      streamKey: sessionCode,
      occurredAt: session.updatedAt,
      outcome: session.lastRunStatus ?? "COMPLETED",
      metadata: {
        appCode: session.appCode ?? "unavailable",
        taskCode: session.taskCode ?? "unavailable",
        localDirectory: "/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/chat-management/base",
        branchName: "feature/clouds3n/chat-management/base",
      },
    },
    {
      id: "EVT-2",
      type: "agent.skill.used",
      eventName: "agent.skill.used",
      runCode: session.lastRunCode,
      streamKey: sessionCode,
      occurredAt: session.updatedAt,
      outcome: "COMPLETED",
      metadata: {
        skillName: "eliteforge-codex-superpower",
      },
    },
  ];
}

export function createMockChatSession(input: CreateChatSessionInput): ChatSession {
  const now = new Date().toISOString();
  const id = Date.now();
  const session: ChatSession = {
    id,
    sessionCode: `SESSION-${id}`,
    workspaceCode: input.workspaceCode,
    userCode: "USER-AUTH",
    appCode: input.appCode ?? null,
    taskCode: input.taskCode ?? null,
    title: input.title?.trim() || "New chat",
    status: "ACTIVE",
    lastRunCode: null,
    lastRunStatus: null,
    createdAt: now,
    updatedAt: now,
  };
  sessions.unshift(session);
  messagesBySession[session.sessionCode] = [];
  return session;
}

export function archiveMockChatSession(sessionCode: string): ChatSession {
  const session = sessions.find((item) => item.sessionCode === sessionCode);
  if (!session) {
    throw new Error("chat session was not found");
  }

  session.status = "ARCHIVED";
  session.updatedAt = new Date().toISOString();
  return { ...session };
}

export function deleteMockChatSession(sessionCode: string): ChatSession {
  const sessionIndex = sessions.findIndex((item) => item.sessionCode === sessionCode);
  if (sessionIndex < 0) {
    throw new Error("chat session was not found");
  }

  const [session] = sessions.splice(sessionIndex, 1);
  delete messagesBySession[sessionCode];
  return { ...session };
}

export function updateMockChatSession(sessionCode: string, input: UpdateChatSessionInput): ChatSession {
  const session = sessions.find((item) => item.sessionCode === sessionCode);
  if (!session) {
    throw new Error("chat session was not found");
  }

  const title = input.title.trim();
  if (!title) {
    throw new Error("session title is required");
  }

  session.title = title;
  session.updatedAt = new Date().toISOString();
  return { ...session };
}

export function createMockChatSendResult(session: ChatSession, input: SendChatMessageInput): ChatSendResult {
  const now = new Date().toISOString();
  const runCode = `RUN-${Date.now()}`;
  const finalOutput = `**Recorded in ${session.sessionCode}:**\n\n${input.content}`;
  return {
    session: {
      ...session,
      lastRunCode: runCode,
      lastRunStatus: "COMPLETED",
      updatedAt: now,
    },
    run: {
      runCode,
      conversationCode: session.sessionCode,
      workspaceCode: input.workspaceCode,
      status: "COMPLETED",
      finalOutput,
      stepCount: 1,
    },
    messages: [
      {
        id: Date.now(),
        messageCode: `MSG-${Date.now()}`,
        sessionCode: session.sessionCode,
        runCode,
        role: "USER",
        channel: session.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        actor: "User",
        content: input.content,
        createdAt: now,
      },
      {
        id: Date.now() + 1,
        messageCode: `MSG-${Date.now() + 1}`,
        sessionCode: session.sessionCode,
        runCode,
        role: "ASSISTANT",
        channel: session.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        actor: "Assistant",
        content: finalOutput,
        createdAt: now,
      },
    ],
  };
}
