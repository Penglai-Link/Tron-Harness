# Type: PROCEDURE

Use `PROCEDURE` for a reusable sequence of actions that future agents should execute when a trigger recurs.

Do not use it for one-off task plans, temporary commands, broad principles, project decisions, or unvalidated ideas.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Procedure: <procedure name>.

Trigger:
- <when to run this procedure>

Prerequisites:
- <required context, permissions, files, state, or inputs>

Steps:
1. <step>
2. <step>

Validation:
- <how to prove the procedure worked>

Fallback:
- <what to do if the procedure cannot be completed safely>

Do not apply when:
- <situations where this procedure is unsafe or irrelevant>

Evidence:
- <brief evidence summary that points to source_refs>
```

Steps must be reusable. Replace incidental command output with durable action intent and validation.

## metadata.typeSpecific

Required fields:

- `trigger`: condition that starts the procedure.
- `successCriteria`: observable result that means the procedure succeeded.
- `validation`: checks, tests, review evidence, or inspection needed.
- `rollbackOrFallback`: safe fallback when the procedure fails or prerequisites are missing.
- `prerequisites`: non-empty list of required conditions or inputs.
- `steps`: ordered list with at least two steps.

## semantic_key Pattern

Use:

```text
procedure:<scope-code>:<domain-slug>:<trigger-slug>:<procedure-name-slug>
```

Rules:

- Use a domain segment such as `frontend-tests`, `skill-validation`, or `deployment`.
- Key by reusable trigger and procedure name, not by a single run date.
- If the procedure changes materially, update the existing memory rather than creating a near-duplicate.

## source_refs Requirement

`source_refs` must include validated operational evidence: project instruction, runbook, accepted implementation report, repeated successful execution, or a reviewed failure recovery. A single failed attempt is not enough unless the memory is explicitly a procedure to avoid that failure and has validation.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "PROCEDURE"
title: "Review a proposed memory before approval"
semantic_key: "procedure:memory-governance:memory-review:approval:quality-gate"
source_refs:
  - type: "memory_policy"
    id: "long-term-memory-quality-standard"
    locator: "approval quality gate"
    observed_at: "2026-06-03"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Long-term memory approval review"
  purpose: "prevent-low-quality-approved-memory"
  typeSpecific:
    trigger: "A proposed memory may become approved, active, or injectable."
    successCriteria: "The record has stable subject, approved type, valid text blocks, evidence, dedupe result, and conflict handling."
    validation: "Check the shared envelope plus selected type reference before approval."
    rollbackOrFallback: "If any gate fails, keep the memory candidate or rejected and record the reason."
    prerequisites:
      - "The proposed memory record is available."
      - "Evidence source refs are available."
      - "The selected memory type is known."
    steps:
      - "Confirm scope, stable subject, future purpose, and evidence."
      - "Check the selected type reference for text structure and required typeSpecific fields."
      - "Search the same scope, subject, and purpose for duplicates or conflicts."
      - "Approve only if evidence and conflict gates pass; otherwise keep candidate or rejected."
text: |
  Procedure: Review a proposed long-term memory before approval.

  Trigger:
  - A proposed memory may become approved, active, or injectable.

  Prerequisites:
  - The proposed memory record, evidence refs, and selected memory type are available.

  Steps:
  1. Confirm scope, stable subject, future purpose, and evidence.
  2. Check the selected type reference for text structure and required typeSpecific fields.
  3. Search the same scope, subject, and purpose for duplicates or conflicts.
  4. Approve only if evidence and conflict gates pass; otherwise keep candidate or rejected.

  Validation:
  - The proposed record passes envelope, type-format, evidence, dedupe, and conflict checks.

  Fallback:
  - If any gate fails, keep the memory candidate or rejected and record the reason.

  Do not apply when:
  - The item is a transient task note, raw output, or unsupported claim.

  Evidence:
  - The referenced memory policy defines the approval quality gate.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "PROCEDURE"
title: "Run stuff"
semantic_key: "procedure:run"
source_refs:
  - type: "terminal"
    id: "last-command"
metadata:
  typeSpecific:
    trigger: "Whenever working"
    successCriteria: "Looks okay"
    validation: "Maybe no error"
    rollbackOrFallback: ""
    prerequisites: []
    steps:
      - "Run commands"
text: |
  Run some commands and see what happens.
```

Problems: not reusable, no prerequisites, not enough steps, no fallback, no precise validation, and source is only a transient terminal event.
