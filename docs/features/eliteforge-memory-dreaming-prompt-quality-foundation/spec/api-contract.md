# API Overview

status: approved

This feature changes local CLI contracts only. It does not add HTTP endpoints.

All script commands keep the existing JSON response shape:

```json
{
  "code": "OK",
  "message": "",
  "data": {}
}
```

# Endpoints

## memory_dreaming.py task-detail

Returns task context plus prompt contract.

Required response data:

- `task`
- `inputs`
- `promptVersion`
- `promptHash`
- `taskPrompt`
- `outputSchema`
- `requirements`
- `previewPolicy`

Raw source preview is redacted by default. Local raw preview requires an explicit opt-in flag.

## memory_dreaming.py task-complete

Accepts task output JSON and validates V2 structure.

Required validation:

- provenance envelope matches the task and prompt;
- required evidence refs are resolvable;
- Light empty output includes `noPointReason`;
- Light memory points create point-level REM tasks;
- REM output references exactly one source memory point;
- Deep output is only accepted when linked to a REM decision that requested it;
- `memoryMcpCalls` is zero.

## memory_dreaming.py finalize

Produces a final report with prompt versions, task counts, accepted artifacts, rejected artifacts, and quality-relevant warnings. It must not include raw real-session material.

## memory_dreaming.py cleanup-temp

Removes script-owned temporary outputs. It must refuse to delete arbitrary paths outside its known artifact roots.

## memory_dreaming_quality.py evaluate

Runs deterministic fixture evaluation.

Inputs:

- `--fixture-root`
- `--artifact-root`
- optional `--scenario`

Outputs:

- per-scenario pass/fail;
- metric summary;
- mutant rejection summary;
- machine-readable JSON report.

## memory_dreaming_live_smoke.py

Creates or describes a private sampled Codex home for live smoke validation.

Inputs:

- `--codex-home`
- `--artifact-root`
- `--sample-size`
- `--seed`
- optional `--dry-run`

Outputs:

- private local sample manifest;
- aggregate-only report template;
- no committed raw snippets, source hashes, or absolute private paths.

# Auth And Permissions

- All commands are local user operations.
- Live smoke requires an explicit local `--codex-home`.
- No network or Memory MCP write is required in V1.

# Error Contract

Errors use the same JSON response shape with a non-OK `code`, a concise `message`, and structured `data` when useful.

Representative codes:

- `INVALID_TASK_OUTPUT`
- `INVALID_PROVENANCE`
- `UNRESOLVABLE_EVIDENCE`
- `UNSAFE_PREVIEW`
- `UNSAFE_ARTIFACT_ROOT`
- `QUALITY_EVALUATION_FAILED`

# Frontend Backend Boundary

Not applicable. No frontend or backend service boundary changes.

# Database And Transaction Boundary

No database transaction exists. File writes should be atomic enough for local CLI use by writing complete JSON artifacts and avoiding partial accepted outputs.

# Mock Or Adapter Expectations

- Synthetic Codex homes must be fixture-owned and deterministic.
- Memory MCP state is represented by fixture JSON only.
- The evaluator must reject outputs that claim live Memory MCP calls in V1.
