# REM Quality

## Goal

Govern exactly one Light point. Produce one evidence-based decision against existing memory state.

## Hard Rules

- Single point only: never batch, merge, or rank multiple Light points in one REM result.
- Run a second pass before final output: re-check evidence, scope, subject, purpose, decision, and execution state.
- Before any create or reinforce decision, check for duplicate or conflicting memory with the same `scope`, `subject`, and `purpose`.
- Governance-history read intent must explicitly cover `ACTIVE`, `CANDIDATE`, and `REJECTED`; default Memory MCP search only covers `ACTIVE+APPROVED`.
- If an equivalent same-purpose memory is known from task material or Memory Tool results, reinforce or reject instead of creating a duplicate.
- If conflict, stale ACTIVE replacement, merge, broad structural change, or unsafe ambiguity remains, return `NEED_DEEP_REVIEW`.
- Reject polluted, stale, overbroad, low-value, weakly evidenced, or tool-output-only points.
- Preferred decision vocabulary: `CREATE_CANDIDATE`, `REINFORCE_CANDIDATE`, `REINFORCE_ACTIVE`, `ACTIVE_EVOLUTION_PROPOSAL`, `REJECT_POINT`, `RECHECK_REJECTED`, `NO_WRITE`, `NEED_DEEP_REVIEW`; the current `outputSchema` remains authoritative.
- Writes are only through `memory_save`; after a successful write, record returned `memoryCode`, `rag`, and tool result as required by the schema.
- Do not claim a write, reinforcement, rejection, active state, `memoryCode`, or `rag` unless a tool call returned it.

## Decision Fit

- `CREATE_CANDIDATE`: no same `scope`/`subject`/`purpose` memory exists; evidence is durable and useful.
- `REINFORCE_CANDIDATE`: equivalent candidate exists; new evidence improves confidence or precision.
- `REINFORCE_ACTIVE`: active memory remains valid; new evidence confirms it without changing meaning.
- `ACTIVE_EVOLUTION_PROPOSAL`: active memory likely needs a scoped change, but REM must not apply it.
- `REJECT_POINT`: point is unsafe, duplicate, polluted, stale, weak, or too local.
- `RECHECK_REJECTED`: rejected memory may now be valid; verify before any new candidate.
- `NO_WRITE`: evidence is true but not worth storing or does not change governance state.
- `NEED_DEEP_REVIEW`: REM cannot safely resolve conflict, merge, stale active state, or structural risk.

## Examples

Good:

- Existing candidate has same `scope`, `subject`, `purpose`; new evidence confirms it -> `REINFORCE_CANDIDATE`.
- Point contradicts ACTIVE memory and both have credible evidence -> `NEED_DEEP_REVIEW`.
- Authorized read material shows a prior `REJECTED` equivalent with the same weakness still present -> `REJECT_POINT`.

Bad:

- Propose another candidate after finding same `scope`, `subject`, `purpose`.
- Claim ACTIVE meaning changed during REM without evidence and a returned tool result.
- Check only ACTIVE memory, miss rejected history, then recreate the same bad point.
