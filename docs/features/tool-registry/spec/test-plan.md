# Test Plan: Tool Registry

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-06-02

## Proof Rows

- AC-TOOL-01 | layer: backend | case: code-backed tool catalog exposes domains and concrete tools | red: catalog endpoints return empty or unstable definitions, omit JSON Schemas, or drift from registered tool metadata | green: backend tests prove domain and tool list endpoints return stable code-backed definitions with expected domain summary fields and concrete tool schemas | interaction: call tool catalog endpoints -> verify domain summaries and tool details | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-02 | layer: backend | case: workspace internal-agent policy resolves defaults, overrides, assignments, and reset without regressing system-agent APIs | red: `main-agent` defaults do not resolve, overrides do not persist, reset does not restore defaults, or system-agent endpoints change semantics | green: backend tests prove default `main-agent` policy resolves to all tools/direct-list/sticky, selected-tools overrides persist correctly, reset removes overrides, and system-agent endpoints still pass | interaction: fetch internal-agent policy -> patch overrides -> replace assignments -> reset -> verify effective response and system-agent parity | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-03 | layer: backend | case: main-agent runtime preparation derives effective tools from internal-agent policy | red: main-agent runs still depend solely on request-provided tools, or direct-list materialization omits effective assigned tools | green: backend tests prove main-agent invocation with no explicit request tools derives effective assigned concrete tools from policy, and domain-discovery runtime state retains the expansion seam without breaking current flows | interaction: invoke main-agent without explicit tools -> inspect runtime request preparation -> change workspace internal-agent policy -> re-invoke -> verify effective tool set changes | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-04 | layer: browser | case: `/settings/tools` shows the platform tool catalog | red: route missing, empty state despite registered tools, or tool metadata missing domain/schema cues | green: browser smoke proves the tools page renders domain-aware tool catalog rows and details from the configured data source | interaction: open `/settings/tools` -> search/filter -> inspect tool row/detail content | command_ref: CMD-TOOL-BROWSER.
- AC-TOOL-05 | layer: browser | case: workspace `/agents` shows internal-agent controls plus existing system-agent controls | red: internal-agent section missing, system-agent section removed/regressed, or tool policy controls fail to render/update | green: browser smoke proves `/agents` shows `main-agent` internal policy controls and still shows system-agent rows with existing behavior | interaction: open `/agents` -> inspect internal-agent section -> change policy controls -> verify feedback -> inspect system-agent section still works | command_ref: CMD-TOOL-BROWSER.

## Command Index

- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: backend compiles cleanly.
- CMD-TOOL-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend/runtime/persistence changes | expect: tool-registry backend tests pass with existing suite.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: tools page and workspace agent page build cleanly.
- CMD-TOOL-BROWSER | command: rtk make fe-test TEST='agent-management-smoke.spec.ts' | cost: slow/browser | when: verify/final | rerun: affected frontend page, mock, or API wiring changes | expect: workspace agent management and tools-page smoke flow pass or are updated to the new scope.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/tool-registry --phase final | cost: medium | when: final | rerun: after doc/task/report/orchestration edits | expect: feature docs validate end to end.

## Evidence Rules

- Every `AC-TOOL-*` row must have final evidence in reports.
- AC-TOOL-02 evidence must explicitly mention that existing system-agent APIs still work.
- AC-TOOL-03 evidence must explicitly describe derived effective tools when request tools are absent.
- Browser evidence must list visible controls, user actions, and feedback.
