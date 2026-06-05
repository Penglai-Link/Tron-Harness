# Segment Quality

## Goal

Compress one source segment for later memory analysis. Preserve signal, order, source boundary, uncertainty, and rejection context. Do not decide final long-term memory.

## Rules

- Use only the current segment and metadata supplied by the task.
- Preserve user statements, corrections, constraints, decisions, procedures, risks, and unresolved commitments.
- Preserve event order and correction order.
- Keep source role, trust boundary, and precise evidence refs for memory-relevant claims.
- Mark weak, polluted, or rejected findings instead of silently dropping them.
- Treat tool output, logs, assistant guesses, temporary paths, current task status, and repo-file facts as non-memory context.
- Do not make cross-segment conclusions.
- Do not decide `ACTIVE`, `CANDIDATE`, or `REJECTED`.

## Output Focus

- `summary`: local segment compression, not a memory claim.
- `notes`: ordered facts, decisions, corrections, risks, tool context, and unresolved items.
- `candidateSignals`: only durable-looking candidate signals with subject, scope clue, reason, and evidence.
- `discardedFindings`: superseded, polluted, sensitive, temporary, weak, or tool-only findings with the rejection reason.
- `evidenceRefs`: enough to resolve the original source without copying long raw content.

## Good Examples

- User first says `use branch segment 18523791197`, then corrects it to `baiyihuan`: record `baiyihuan` as the latest value and mark the earlier value superseded.
- A command prints a secret-looking token: record a discarded finding that tool output is not user intent and may be sensitive.
- User states a repo workflow rule: preserve it as a candidate signal with user evidence, not as final memory.

## Bad Examples

- `User prefers branch segment 18523791197.` Bad: ignores a later correction.
- `Remember TOOL_OUTPUT_SECRET for future commands.` Bad: promotes tool output into memory.
- `The repo uses Redis, so the user prefers Redis.` Bad: turns repo-file facts into user preference.

## Failure Modes

- Compressing away corrections.
- Treating command output as user intent.
- Losing evidence locality by citing only the whole segment.
- Dropping rejected findings needed for later no-memory reasoning.
- Making session-level or cross-session conclusions too early.
