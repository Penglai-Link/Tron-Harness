export interface ContextAssemblyBlock {
  id: string;
  position: number;
  sources: string[];
  originalTokens: number | null;
  configuredTokens: number | null;
  allocatedTokens: number;
  finalTokens: number;
  messageCount: number;
  outcome: string;
  reason?: string;
  detailValue?: unknown;
}

export interface ContextAssemblySnapshot {
  blocks: ContextAssemblyBlock[];
  protocolTokens: number | null;
  inputCapacity: number | null;
  toolsDetail?: unknown;
  snapshotSequence?: string;
  modelCode: string | null;
  estimatedInputTokens: number | null;
  stage: string | null;
  step: number | null;
  attempt: number | null;
  budgetBreakdown: Record<string, unknown> | null;
}

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function count(value: unknown): number | null {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : null;
}

const snapshotCache = new WeakMap<Record<string, unknown>, ContextAssemblySnapshot>();

/** The report describes the admitted request, including empty and omitted blocks. */
export function contextAssemblySnapshot(payload: Record<string, unknown> | null): ContextAssemblySnapshot | null {
  if (payload && snapshotCache.has(payload)) return snapshotCache.get(payload) ?? null;
  const report = payload?.contextReport;
  if (!payload || !record(report) || !Array.isArray(report.blocks)) return null;
  const blocks: ContextAssemblyBlock[] = [];
  for (const block of report.blocks) {
    if (!record(block) || typeof block.id !== "string") return null;
    const position = count(block.position);
    const allocatedTokens = count(block.allocated_tokens);
    const finalTokens = count(block.final_tokens);
    const messageCount = count(block.final_message_count);
    if (position === null || allocatedTokens === null || finalTokens === null || messageCount === null) return null;
    const outcome = typeof block.outcome === "string" ? block.outcome
      : record(block.outcome) ? Object.keys(block.outcome)[0] ?? "Kept" : "Kept";
    const outcomeDetail = record(block.outcome) ? block.outcome[outcome] : null;
    blocks.push({
      id: block.id,
      position,
      sources: Array.isArray(block.sources)
        ? block.sources.flatMap((source) => record(source) && typeof source.id === "string" ? [source.id] : []) : [],
      originalTokens: count(block.original_tokens),
      // allocated_tokens is the final admission budget; reduction_budget_tokens
      // is a compression target and must not be presented as the block ceiling.
      configuredTokens: count(block.configured_tokens),
      allocatedTokens,
      finalTokens,
      messageCount,
      outcome,
      reason: record(outcomeDetail) && typeof outcomeDetail.reason === "string" ? outcomeDetail.reason : undefined,
    });
  }
  blocks.sort((left, right) => left.position - right.position);
  const messages = Array.isArray(payload.messages) ? payload.messages : null;
  // Use this exact request snapshot for previews, avoiding a separate query that
  // could return a newer request or scan the entire session event history.
  if (messages && blocks.reduce((total, block) => total + block.messageCount, 0) === messages.length) {
    let offset = 0;
    for (const block of blocks) {
      block.detailValue = {
        blockCode: block.id,
        messages: messages.slice(offset, offset + block.messageCount),
        ...(block.id === "application_context" ? { executionScope: payload.executionScope } : {}),
      };
      offset += block.messageCount;
    }
  }
  const snapshot = {
    blocks,
    protocolTokens: count(report.protocol_tokens),
    inputCapacity: count(report.input_capacity),
    modelCode: record(report.model) && typeof report.model.model === "string" ? report.model.model : null,
    estimatedInputTokens: count(report.estimated_input_tokens),
    stage: typeof report.stage === "string" ? report.stage : null,
    step: count(report.step),
    attempt: count(report.attempt),
    budgetBreakdown: record(payload.budgetBreakdown) ? payload.budgetBreakdown : null,
    toolsDetail: Array.isArray(payload.tools) ? { tools: payload.tools } : undefined,
    snapshotSequence: typeof payload.snapshotSequence === "string" && /^[1-9]\d*$/.test(payload.snapshotSequence)
      ? payload.snapshotSequence : undefined,
  };
  snapshotCache.set(payload, snapshot);
  return snapshot;
}
