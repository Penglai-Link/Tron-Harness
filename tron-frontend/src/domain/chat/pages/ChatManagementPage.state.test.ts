// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

vi.hoisted(() => {
  Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
    value: () => null,
  });
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      localStorage: {
        getItem: () => null,
        setItem: () => undefined,
      },
      location: {
        origin: "http://localhost",
        pathname: "/chats",
        search: "",
      },
    },
  });
});

import {
  buildReportedContextMemoryBlocks,
  defaultChatModelOption,
  latestContextBudgetBreakdownFromEvents,
  latestShortTermMemoryRuntimeNotice,
} from "./ChatManagementPage";
import { contextAssemblySnapshot } from "../model/contextAssemblyReport";

describe("saved plugin context budgets", () => {
  it("uses the shared catalog for native, plugin and protocol descriptions in live and saved reports", () => {
    const snapshot = contextAssemblySnapshot({ contextReport: {
      input_capacity: 1_000, protocol_tokens: 10,
      blocks: [{ id: "skills.catalog", position: 0, sources: [{ id: "skills.catalog" }], allocated_tokens: 20, final_tokens: 20, final_message_count: 1 }],
    } })!;
    const catalog = new Map([
      ["skills.catalog", { id: "skills.catalog", displayName: "Plugin-owned name", description: "Plugin-owned description", updateMode: "Replace" as const, providerId: "skills" }],
      ["protocol", { id: "protocol", displayName: "Tool definitions", description: "Protocol description", updateMode: "Replace" as const, providerId: null }],
    ]);
    const rows = buildReportedContextMemoryBlocks(snapshot, key => key, catalog);
    expect(rows[0]).toMatchObject({ name: "Plugin-owned name", description: "Plugin-owned description", updateMode: "Replace" });
    expect(rows[1]).toMatchObject({ name: "Tool definitions", description: "Protocol description" });
    expect(buildReportedContextMemoryBlocks(JSON.parse(JSON.stringify(snapshot)), key => key, catalog)).toEqual(rows);
  });
  it.each([776_601, 0])("retains the saved protocol ceiling for live and historical reports (%s)", (ceiling) => {
    const snapshot = contextAssemblySnapshot({
      snapshotSequence: "4",
      budgetBreakdown: { toolSchemaBudgetTokens: ceiling },
      contextReport: { input_capacity: 1_294_336, protocol_tokens: 303, blocks: [] },
    })!;
    const rows = buildReportedContextMemoryBlocks(snapshot, (key) => key);
    expect(rows[0]).toMatchObject({ code: "protocol", tokens: 303, limitTokens: ceiling, snapshotSequence: "4" });
    expect(buildReportedContextMemoryBlocks(JSON.parse(JSON.stringify(snapshot)), (key) => key)).toEqual(rows);
  });
  it.each([1_300_000, null])("does not treat an exact admission allocation as a full window (%s)", (capacity) => {
    const snapshot = contextAssemblySnapshot({ contextReport: {
      input_capacity: capacity,
      protocol_tokens: 5600,
      blocks: [{ id: "agent_directory", position: 0, sources: [], original_tokens: 690,
        allocated_tokens: 690, configured_tokens: 2048, reduction_budget_tokens: 690, final_tokens: 690,
        final_message_count: 1, outcome: "Kept" }],
    } });
    expect(snapshot).not.toBeNull();
    const blocks = buildReportedContextMemoryBlocks(snapshot!, (key) => key);
    expect(blocks[0]).toMatchObject({ order: 1, tokens: 690, allocationTokens: 690, limitTokens: 2048 });
    expect(blocks[1]).toMatchObject({ order: 2, tokens: 5600, allocationTokens: null, limitTokens: null });
    expect(blocks[0]!.tokens / blocks[0]!.limitTokens!).toBeLessThan(0.5);
  });
});

describe("chat model default selection", () => {
  const fixedDefault = {
    enabled: true,
    isDefault: true,
    modelSelection: { kind: "Fixed" as const, modelCode: "MODEL-1" },
    source: "internal-agent" as const,
  };
  const autoRoute = {
    enabled: true,
    isDefault: false,
    modelSelection: {
      kind: "ProviderRoute" as const,
      providerCode: "PersonAi" as const,
      routeCode: "ChatAuto" as const,
    },
    source: "internal-agent" as const,
  };

  it("prefers enabled Person-AI Auto when a fixed model is marked as default", () => {
    expect(defaultChatModelOption([fixedDefault, autoRoute])).toBe(autoRoute);
  });

  it("keeps the configured model default when the Auto route is unavailable", () => {
    expect(defaultChatModelOption([
      fixedDefault,
      { ...autoRoute, enabled: false },
    ])).toBe(fixedDefault);
  });
});

describe("plugin assembly report", () => {
  it("labels instruction sources without treating them as individual messages", () => {
    const snapshot = contextAssemblySnapshot({ contextReport: { input_capacity: 32000, blocks: [{
      id: "framework.instructions", position: 0, allocated_tokens: 6000,
      final_tokens: 5000, final_message_count: 5,
      sources: [{ id: "framework.instructions" }, { id: "agent.directory-guidance" }, { id: "skills.guidance" }, { id: "a2ui.guidance" }],
    }] } })!;
    const block = buildReportedContextMemoryBlocks(snapshot, key => key)[0];
    expect(block.stat).toBe("chat.shortTermMemory.contextPanel.messageCount");
    expect(block.sources?.map(source => source.name)).toEqual([
      "chat.shortTermMemory.contextPanel.baseInstructionsName",
      "chat.shortTermMemory.contextPanel.directoryGuidanceName",
      "chat.shortTermMemory.contextPanel.skillsGuidanceName",
      "chat.shortTermMemory.contextPanel.a2uiGuidanceName",
    ]);
  });
  it("renders the shared page contributor in live and saved Dude assembly order", () => {
    const blocks = ["framework.instructions", "framework.history", "agent_directory", "skills.catalog", "page_context", "dude_environment"]
      .map((id, position) => ({ id, position, allocated_tokens: 100, configured_tokens: 2048, final_tokens: 30, final_message_count: 1 }));
    const live = contextAssemblySnapshot({ snapshotSequence: "80", contextReport: { input_capacity: 32000, blocks } })!;
    const saved = JSON.parse(JSON.stringify(live));
    const rows = buildReportedContextMemoryBlocks(live, key => key);
    expect(rows.map(row => row.code)).toEqual(blocks.map(block => block.id));
    expect(rows.find(row => row.code === "page_context")).toMatchObject({
      name: "chat.shortTermMemory.contextPanel.pageContextName", detailCode: "page_context", snapshotSequence: "80", order: 5,
    });
    expect(buildReportedContextMemoryBlocks(saved, key => key)).toEqual(rows);
  });
  it("uses report positions and retains every plugin block after saving", () => {
    const payload={contextReport:{input_capacity:1300000,protocol_tokens:303,estimated_input_tokens:8600,model:{model:"fixture-model"},stage:"chat",step:1,attempt:1,blocks:[
      {id:"framework.history",position:7,allocated_tokens:1200000,configured_tokens:1300000,final_tokens:252,final_message_count:1},
      {id:"knowledge_base",position:4,allocated_tokens:0,configured_tokens:1200,final_tokens:0,final_message_count:0},
      {id:"agent_directory",position:5,allocated_tokens:690,configured_tokens:2048,final_tokens:690,final_message_count:1},
      {id:"framework.instructions",position:0,allocated_tokens:7362,configured_tokens:16000,final_tokens:7362,final_message_count:6},
    ]},snapshotSequence:"52"};
    const live=contextAssemblySnapshot(payload)!;
    const saved=JSON.parse(JSON.stringify(live));
    const rows=buildReportedContextMemoryBlocks(live,key=>key);
    expect(buildReportedContextMemoryBlocks(saved,key=>key)).toEqual(rows);
    expect(rows.map(({code})=>code)).toEqual(["framework.instructions","knowledge_base","agent_directory","framework.history","protocol"]);
    expect(rows.map(({order})=>order)).toEqual([1,2,3,4,5]);
    expect(rows.every(row=>row.snapshotSequence==="52")).toBe(true);
    expect(rows[2]).toMatchObject({tokens:690,allocationTokens:690,limitTokens:2048});
  });
  it("keeps missing historical ceilings unknown instead of borrowing the model capacity", () => {
    const snapshot=contextAssemblySnapshot({contextReport:{input_capacity:1300000,protocol_tokens:0,blocks:[{id:"custom-plugin",position:1,allocated_tokens:690,final_tokens:12,final_message_count:1}]}})!;
    expect(buildReportedContextMemoryBlocks(snapshot,key=>key)[0]).toMatchObject({name:"custom-plugin",tokens:12,allocationTokens:690,limitTokens:null});
    expect(contextAssemblySnapshot({budgetBreakdown:{estimatedInputTokens:100}})).toBeNull();
  });
  it("reads a live context budget from an AG-UI custom event", () => {
    const breakdown = latestContextBudgetBreakdownFromEvents([{
      name: "tron.short_term_memory.context.assembled",
      threadId: "SESSION-1",
      timestamp: 100,
      type: "CUSTOM",
      value: {
        budgetBreakdown: {
          estimatedInputTokens: 12_000,
          effectiveInputBudget: 28_000,
          toolSchemaBudgetTokens: 6_000,
          toolSchemaTokens: 3_000,
        },
      },
    }]);

    expect(breakdown).toMatchObject({
      estimatedInputTokens: 12_000,
      toolSchemaBudgetTokens: 6_000,
      toolSchemaTokens: 3_000,
    });
  });

  it("shows the latest automatic compaction lifecycle from AG-UI", () => {
    expect(latestShortTermMemoryRuntimeNotice([
      {
        name: "tron.short_term_memory.compaction.started",
        timestamp: 100,
        type: "CUSTOM",
        value: { compactionInputTokenEstimate: 24_000, maxInputTokens: 28_000 },
      },
      {
        name: "tron.short_term_memory.compaction.completed",
        timestamp: 200,
        type: "CUSTOM",
        value: { checkpointMessageCode: "ATM-1" },
      },
    ])).toMatchObject({
      kind: "compaction_completed",
      timestamp: 200,
    });
  });

  it("keeps a tool-schema budget error until a later run succeeds", () => {
    const error = {
      code: "tool_schema_over_budget",
      message: "TOOL_SCHEMA_OVER_BUDGET: estimatedTokens=27294, limitTokens=19200, toolCount=93; tool exposure was not changed",
      timestamp: 100,
      type: "RUN_ERROR",
    };
    expect(latestShortTermMemoryRuntimeNotice([error])).toMatchObject({
      estimatedTokens: 27_294,
      kind: "tool_schema_over_budget",
      limitTokens: 19_200,
      toolCount: 93,
    });
    expect(latestShortTermMemoryRuntimeNotice([
      error,
      { timestamp: 200, type: "RUN_FINISHED" },
    ])).toBeNull();
  });
});


it("preserves uncapped stable blocks before history in live and saved reports", () => {
  const snapshot=contextAssemblySnapshot({contextReport:{input_capacity:100000,protocol_tokens:0,blocks:[
    {id:"framework.instructions",position:0,configured_tokens:null,allocated_tokens:20000,final_tokens:20000,final_message_count:1},
    {id:"tool_domains",position:1,configured_tokens:null,allocated_tokens:18000,final_tokens:18000,final_message_count:1},
    {id:"framework.history",position:2,configured_tokens:100000,allocated_tokens:62000,final_tokens:1200,final_message_count:3},
  ]}})!;
  const rows=buildReportedContextMemoryBlocks(snapshot,key=>key);
  expect(rows.slice(0,3).map(row=>row.code)).toEqual(["framework.instructions","tool_domains","framework.history"]);
  expect(rows.slice(0,2).map(row=>row.limitTokens)).toEqual([null,null]);
  expect(buildReportedContextMemoryBlocks(JSON.parse(JSON.stringify(snapshot)),key=>key)).toEqual(rows);
});
