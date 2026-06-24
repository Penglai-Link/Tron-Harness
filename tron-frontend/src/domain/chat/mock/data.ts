import type {
  ChatAguiEvent,
  ChatMessage,
  ChatSendResult,
  ChatSessionCompaction,
  ChatSessionGoalView,
  ChatSession,
  ChatSessionGroup,
  ChatSessionTodo,
  AddChatSessionGoalCriterionInput,
  ChatSessionGoalCommand,
  CreateChatCompactionInput,
  CreateChatSessionInput,
  RemoveChatSessionGoalCriterionInput,
  SendChatMessageInput,
  SetChatSessionGoalInput,
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
    sourceCode: "TASK-8842",
    sourceType: "Task",
    agentCode: "main-agent",
    systemAgentSessionId: null,
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
    sourceCode: "TASK-8842",
    sourceType: "Task",
    agentCode: "codex",
    systemAgentSessionId: "thr_codex_mock_39",
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
    sourceCode: null,
    sourceType: "Manual",
    agentCode: "main-agent",
    systemAgentSessionId: null,
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
      inputTokens: 64,
      outputTokens: 42,
      totalTokens: 106,
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
      inputTokens: 38,
      outputTokens: 23,
      totalTokens: 61,
      createdAt: "2026-05-27T09:50:00Z",
    },
  ],
  "SESSION-UNASSIGNED": [],
};

const todosBySession: Record<string, ChatSessionTodo[]> = {
  "SESSION-8842": [
    {
      id: 735210000000000001,
      todoCode: "TODO-MOCK-1",
      sessionCode: "SESSION-8842",
      workspaceCode: "TRON_CORE",
      position: 0,
      content: "Validate transcript storage after the next chat turn",
      status: "in_progress",
      priority: "high",
      createdByRunCode: "RUN-42",
      updatedByRunCode: "RUN-42",
      createdAt: "2026-05-27T10:42:30Z",
      updatedAt: "2026-05-27T10:44:00Z",
    },
    {
      id: 735210000000000002,
      todoCode: "TODO-MOCK-2",
      sessionCode: "SESSION-8842",
      workspaceCode: "TRON_CORE",
      position: 1,
      content: "Review compaction checkpoint output",
      status: "pending",
      priority: "medium",
      createdByRunCode: "RUN-42",
      updatedByRunCode: "RUN-42",
      createdAt: "2026-05-27T10:43:00Z",
      updatedAt: "2026-05-27T10:44:00Z",
    },
  ],
  "SESSION-REFACTOR": [],
  "SESSION-UNASSIGNED": [],
};

const compactionsBySession: Record<string, ChatSessionCompaction | null> = {
  "SESSION-8842": {
    sessionCode: "SESSION-8842",
    checkpointMessageCode: "ATM-MOCK-1",
    status: "completed",
    sourceStartSeq: 1,
    sourceEndSeq: 8,
    tailStartSeq: 5,
    summaryTokenEstimate: 96,
    createdAt: "2026-05-27T10:45:00Z",
  },
  "SESSION-REFACTOR": null,
  "SESSION-UNASSIGNED": null,
};

const goalsBySession: Record<string, ChatSessionGoalView | null> = {
  "SESSION-8842": {
    goal: {
      id: 735220000000000001,
      goalCode: "GOAL-MOCK-1",
      sessionCode: "SESSION-8842",
      workspaceCode: "TRON_CORE",
      userCode: "USER-AUTH",
      taskCode: "TASK-8842",
      activeFamilyCode: null,
      objectiveMarkdown: "Finish the auth latency rollback investigation with evidence.",
      status: "ACTIVE",
      runsUsed: 2,
      maxRuns: 20,
      lastRunCode: "RUN-42",
      lastInputCode: null,
      lastVerdict: "CONTINUE",
      lastReason: "Rollback path is identified; verification evidence is still pending.",
      blockedReason: null,
      requiredUserInput: null,
      consecutiveJudgeFailures: 0,
      createdByRunCode: "RUN-42",
      createdAt: "2026-05-27T10:40:00Z",
      updatedAt: "2026-05-27T10:44:00Z",
      pausedAt: null,
      blockedAt: null,
      completedAt: null,
      cancelledAt: null,
      clearedAt: null,
      metadataJson: null,
    },
    criteria: [
      {
        id: 735221000000000001,
        criterionCode: "CRITERION-MOCK-1",
        goalCode: "GOAL-MOCK-1",
        sessionCode: "SESSION-8842",
        position: 0,
        text: "Rollback path includes the affected regions and a verification step.",
        status: "ACTIVE",
        createdAt: "2026-05-27T10:40:00Z",
        updatedAt: "2026-05-27T10:40:00Z",
        removedAt: null,
      },
    ],
  },
  "SESSION-REFACTOR": null,
  "SESSION-UNASSIGNED": null,
};

export function getChatSessionGroups(): ChatSessionGroup[] {
  return [
    {
      taskCode: "TASK-8842",
      taskTitle: "Auth latency rollback",
      sourceCode: "TASK-8842",
      sourceType: "Task",
      sessions: sessions.filter((session) => session.taskCode === "TASK-8842"),
    },
    {
      taskCode: null,
      taskTitle: "Chat",
      sourceCode: null,
      sourceType: "Manual",
      sessions: sessions.filter((session) => !session.taskCode),
    },
  ];
}

export function getChatMessages(sessionCode: string): ChatMessage[] {
  return messagesBySession[sessionCode] ?? [];
}

export function getMockChatSessionTodos(sessionCode: string): ChatSessionTodo[] {
  return [...(todosBySession[sessionCode] ?? [])];
}

export function getMockChatLatestCompaction(sessionCode: string): ChatSessionCompaction | null {
  const compaction = compactionsBySession[sessionCode] ?? null;
  return compaction ? { ...compaction } : null;
}

export function createMockChatCompaction(input: CreateChatCompactionInput): ChatSessionCompaction {
  const now = new Date().toISOString();
  const messages = messagesBySession[input.sessionCode] ?? [];
  const compaction: ChatSessionCompaction = {
    sessionCode: input.sessionCode,
    checkpointMessageCode: `ATM-MOCK-${Date.now()}`,
    status: "completed",
    sourceStartSeq: messages.length ? 1 : 0,
    sourceEndSeq: messages.length,
    tailStartSeq: Math.max(messages.length - 3, 0),
    summaryTokenEstimate: Math.max(messages.length * 24, 1),
    createdAt: now,
  };
  compactionsBySession[input.sessionCode] = compaction;
  return { ...compaction };
}

export function getMockChatSessionGoal(sessionCode: string): ChatSessionGoalView | null {
  const goal = goalsBySession[sessionCode] ?? null;
  if (!goal) return null;
  return ["ACTIVE", "PAUSED", "BLOCKED"].includes(goal.goal.status) ? cloneGoalView(goal) : null;
}

export function setMockChatSessionGoal(input: SetChatSessionGoalInput): ChatSessionGoalView {
  const session = sessions.find((item) => item.sessionCode === input.sessionCode);
  if (!session) throw new Error("chat session was not found");
  if (!input.objective.trim()) throw new Error("goal objective is required");
  const now = new Date().toISOString();
  const goalCode = `GOAL-MOCK-${Date.now()}`;
  const goal: ChatSessionGoalView = {
    goal: {
      id: Date.now(),
      goalCode,
      sessionCode: input.sessionCode,
      workspaceCode: session.workspaceCode,
      userCode: session.userCode,
      taskCode: session.taskCode,
      activeFamilyCode: null,
      objectiveMarkdown: input.objective.trim(),
      status: "ACTIVE",
      runsUsed: 0,
      maxRuns: input.maxRuns ?? 20,
      lastRunCode: null,
      lastInputCode: null,
      lastVerdict: null,
      lastReason: null,
      blockedReason: null,
      requiredUserInput: null,
      consecutiveJudgeFailures: 0,
      createdByRunCode: null,
      createdAt: now,
      updatedAt: now,
      pausedAt: null,
      blockedAt: null,
      completedAt: null,
      cancelledAt: null,
      clearedAt: null,
      metadataJson: { outerLoopStatus: "not_started", startPolicy: "after_next_run" },
    },
    criteria: (input.criteria ?? []).map((text, index) => ({
      id: Date.now() + index + 1,
      criterionCode: `CRITERION-MOCK-${Date.now()}-${index}`,
      goalCode,
      sessionCode: input.sessionCode,
      position: index,
      text,
      status: "ACTIVE",
      createdAt: now,
      updatedAt: now,
      removedAt: null,
    })),
  };
  goalsBySession[input.sessionCode] = goal;
  return cloneGoalView(goal);
}

export function transitionMockChatSessionGoal(
  input: ChatSessionGoalCommand,
  status: ChatSessionGoalView["goal"]["status"],
): ChatSessionGoalView {
  const goal = goalsBySession[input.sessionCode];
  if (!goal || (input.goalCode && goal.goal.goalCode !== input.goalCode)) {
    throw new Error("session goal was not found");
  }
  goal.goal.status = status;
  goal.goal.updatedAt = new Date().toISOString();
  return cloneGoalView(goal);
}

export function addMockChatSessionGoalCriterion(input: AddChatSessionGoalCriterionInput): ChatSessionGoalView {
  const goal = goalsBySession[input.sessionCode];
  if (!goal || (input.goalCode && goal.goal.goalCode !== input.goalCode)) {
    throw new Error("session goal was not found");
  }
  const now = new Date().toISOString();
  goal.criteria.push({
    id: Date.now(),
    criterionCode: `CRITERION-MOCK-${Date.now()}`,
    goalCode: goal.goal.goalCode,
    sessionCode: input.sessionCode,
    position: goal.criteria.length,
    text: input.text,
    status: "ACTIVE",
    createdAt: now,
    updatedAt: now,
    removedAt: null,
  });
  goal.goal.updatedAt = now;
  return cloneGoalView(goal);
}

export function removeMockChatSessionGoalCriterion(input: RemoveChatSessionGoalCriterionInput): ChatSessionGoalView {
  const goal = goalsBySession[input.sessionCode];
  if (!goal || goal.goal.goalCode !== input.goalCode) {
    throw new Error("session goal was not found");
  }
  const now = new Date().toISOString();
  goal.criteria = goal.criteria.map((criterion) => (
    criterion.criterionCode === input.criterionCode
      ? { ...criterion, status: "REMOVED", removedAt: now, updatedAt: now }
      : criterion
  ));
  goal.goal.updatedAt = now;
  return cloneGoalView(goal);
}

function cloneGoalView(goal: ChatSessionGoalView): ChatSessionGoalView {
  return {
    goal: { ...goal.goal },
    criteria: goal.criteria.map((criterion) => ({ ...criterion })),
  };
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
    agentCode: input.agentCode ?? "main-agent",
    systemAgentSessionId: null,
    title: input.title?.trim() || "New chat",
    status: "ACTIVE",
    lastRunCode: null,
    lastRunStatus: null,
    createdAt: now,
    updatedAt: now,
  };
  sessions.unshift(session);
  messagesBySession[session.sessionCode] = [];
  todosBySession[session.sessionCode] = [];
  compactionsBySession[session.sessionCode] = null;
  goalsBySession[session.sessionCode] = null;
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
  delete todosBySession[sessionCode];
  delete compactionsBySession[sessionCode];
  delete goalsBySession[sessionCode];
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
  const systemAgentSessionId = session.agentCode && session.agentCode !== "main-agent"
    ? session.systemAgentSessionId ?? `ses-${session.sessionCode.toLowerCase()}`
    : null;
  return {
    session: {
      ...session,
      lastRunCode: runCode,
      lastRunStatus: "COMPLETED",
      systemAgentSessionId,
      updatedAt: now,
    },
    run: {
      runCode,
      conversationCode: session.sessionCode,
      workspaceCode: input.workspaceCode,
      status: "COMPLETED",
      finalOutput,
      stepCount: 1,
      tokenUsage: {
        inputTokens: 32,
        outputTokens: 24,
        totalTokens: 56,
      },
      systemAgentSessionId,
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
        inputTokens: 32,
        outputTokens: 24,
        totalTokens: 56,
        createdAt: now,
      },
    ],
  };
}
