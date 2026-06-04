# Acceptance: Response API Parallel Tool Calls

## Acceptance Index

- AC-RPTC-01 [must] [business] Internal-agent runtime treats `maxToolCalls` as the total tool-call budget for the run and `maxParallelToolCalls` as the per-batch concurrency cap, executing oversized tool-call sets in sequential batches instead of failing before any allowed call runs; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RPTC-02 [must] [regression] When pending tool calls exceed the remaining total budget, runtime executes only the remaining allowed calls and then fails on the first disallowed call with the existing max-tool-calls failure contract; proof: @ref:spec/test-plan.md#proof-rows.
- AC-RPTC-03 [must] [interaction] Workspace `/settings/agents` exposes distinct controls and help text for total tool-call budget and parallel batch cap on internal-agent policy forms; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
