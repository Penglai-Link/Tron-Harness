# Test Plan: Tool Registry

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-06-02

## Proof Rows

- AC-TOOL-01 | layer: backend | case: code-backed tool catalog exposes business and coding domains with schema descriptions/examples | red: catalog endpoints return empty or unstable definitions, omit coding domains, omit JSON Schema `description`/`examples`, or drift from registered tool metadata | green: backend tests prove domain and tool list endpoints return stable code-backed definitions with expected domain summary fields, coding tool rows, and concrete tool schemas whose top-level JSON includes `description` and `examples` | interaction: not applicable | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-02 | layer: backend | case: workspace internal-agent policy resolves defaults, overrides, assignments, and reset without regressing system-agent APIs | red: `main-agent` defaults do not resolve, overrides do not persist, reset does not restore defaults, or system-agent endpoints change semantics | green: backend tests prove default `main-agent` policy resolves to all tools/direct-list/sticky, selected-tools overrides persist correctly, reset removes overrides, and system-agent endpoints still pass | interaction: not applicable | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-03 | layer: backend | case: main-agent runtime preparation and coding-tool execution derive effective tools from internal-agent policy | red: main-agent runs still depend solely on request-provided tools, direct-list materialization omits effective assigned tools, or coding tools cannot execute after policy-derived materialization | green: backend tests prove main-agent invocation with no explicit request tools derives effective assigned concrete tools from policy, domain-discovery runtime state retains the expansion seam, and bounded `fs.*`, `search.*`, and `bash.exec` tools execute through registered executors | interaction: not applicable | command_ref: CMD-TOOL-BE-TEST.
- AC-TOOL-04 | layer: frontend | case: `/settings/tools` remains unchanged in this backend-only coding-domain evolution | red: reopened backend work would unexpectedly require frontend page changes | green: this evolution leaves `/settings/tools` untouched and defers any frontend revalidation to a future UI-focused slice | interaction: not applicable | command_ref: CMD-FE-CHECK.
- AC-TOOL-05 | layer: frontend | case: workspace `/agents` remains unchanged in this backend-only coding-domain evolution | red: reopened backend work would unexpectedly require frontend page changes | green: this evolution leaves workspace `/agents` untouched and defers any frontend revalidation to a future UI-focused slice | interaction: not applicable | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: backend compiles cleanly.
- CMD-TOOL-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend/runtime/persistence changes | expect: tool-registry backend tests pass with existing suite.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: tools page and workspace agent page build cleanly.
- CMD-TOOL-BROWSER | command: rtk make fe-test TEST='agent-management-smoke.spec.ts' | cost: slow/browser | when: verify/final | rerun: affected frontend page, mock, or API wiring changes | expect: workspace agent management and tools-page smoke flow pass or are updated to the new scope.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/tool-registry --phase final | cost: medium | when: final | rerun: after doc/task/report/orchestration edits | expect: feature docs validate end to end.

## Evidence Rules

- Every `AC-TOOL-*` row must have final evidence in reports.
- AC-TOOL-02 evidence must explicitly mention that existing system-agent APIs still work.
- AC-TOOL-03 evidence must explicitly describe derived effective tools when request tools are absent and identify at least one successful coding-tool execution path.
- Browser evidence must list visible controls, user actions, and feedback.
