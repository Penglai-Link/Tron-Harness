# Deep Quality

## Goal

Resolve governance cases REM cannot safely decide: conflicts, candidate accumulation, duplicates, expiry, archive, disable, ACTIVE evolution, and structure health.

## Hard Rules

- Cover every requested target; mark each as resolved, proposed, deferred, or no-op with evidence.
- Stay inside the approved scope. Never widen repo-local facts into global preferences or policy.
- Separate decisions for conflict, duplicate, stale, expired, archive, disable, merge, split, evolution, and structure repair.
- Do not batch-change ACTIVE memory unless evidence is explicit.
- Apply ACTIVE evolution only when evidence proves the old meaning is wrong, incomplete, or dangerously imprecise.
- Prefer precise candidate consolidation over broad merging; preserve distinct `scope`, `subject`, and `purpose`.
- Treat large candidate piles as a signal to dedupe, reinforce winners, reject weak entries, or request human review.
- Delete only through `memory_delete`; deletion is soft and mode must be `ARCHIVE` or `DISABLE`.
- Use `ARCHIVE` for obsolete-but-valuable history. Use `DISABLE` for harmful, polluted, misleading, or unsafe memory.
- `memory_save` writes must record returned `memoryCode`, `rag`, and tool result.
- `memory_delete` actions must record returned `memoryCode`, action, status fields, `deleted`, `warnings`, and tool result.
- Do not invent execution, identifiers, status changes, or deletion.
- If evidence is insufficient, produce a recommendation or `NO_WRITE`; do not simulate side effects.

## Decision Pressure

- Conflict: identify winning evidence, losing evidence, unresolved risk, and whether ACTIVE change is proposed or executed.
- Candidate accumulation: group by `scope`/`subject`/`purpose`; reinforce one, reject duplicates, or defer unsafe groups.
- Duplicate: keep the most precise and best-evidenced memory in the proposal; preserve useful evidence through task evidence refs or `source_refs`.
- Expiry: archive stale-but-true context; disable false or harmful memory.
- ACTIVE evolution: require explicit evidence and successful tool results.
- Structure health: fix overloaded scope, vague subject, mixed purpose, or low-value roots through minimal targeted changes.

## Examples

Good:

- Five candidates share `scope`, `subject`, and `purpose`; propose keeping the best-evidenced one, rejecting duplicates, and recording reasons.
- New evidence supersedes ACTIVE memory but is not yet sufficient -> propose ACTIVE evolution, no side effect claimed.
- Memory is obsolete historical context; archive it only with evidence and a returned tool result.

Bad:

- Merge repo-specific behavior into a global user preference.
- Disable memory because it is old, without proving it is harmful or false.
- Change several ACTIVE memories in one pass because they look related.
- Report deletion or ACTIVE update without a returned tool result.
