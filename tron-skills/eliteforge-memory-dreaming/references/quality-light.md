# Light Quality

## Goal

Extract lightweight memory candidates from a session summary. Emit only durable, future-useful candidates; otherwise emit no-memory output.

## Rules

- Keep only memories that are durable, scoped, evidenced, actionable, and useful outside the original task.
- Accept only supported memory types: `USER_PREFERENCE`, `SPACE_RULE`, `WORK_DOMAIN_EXPERIENCE`, `PROJECT_DECISION`, `PROCEDURE`, `RISK`, `FACT`.
- Use only current semantic scopes: `USER`, `WORK_DOMAIN`, and `SPACE`; emit the spelling required by the current `outputSchema` and do not emit old `AGENT` scope.
- `WORK_DOMAIN` needs a recognizable professional domain and reusable value beyond one project or task. Domain names are not scope values by themselves.
- Reject temporary task status, plain file changes, one-off command output, tool/log output as user fact, assistant guesses, stale facts, sensitive text, and broad claims without a stable subject.
- Each point needs memory type, normalized text, governance hints, confidence, type-specific detail, usage boundaries, and resolvable evidence.
- Governance hints must include target scope, subject, purpose, importance hint, source roles, trust boundaries, risk flags, scoring, and `memoryValueScore`.
- `confidence` measures current-session evidence strength; `importanceHint` and `memoryValueScore` measure future reuse value.
- Use the latest supported correction.
- Prefer no memory over weak memory.
- Do not decide final status or claim the point was saved without a real tool result.

## Admission

- `USER_PREFERENCE`: explicit long-term user preference or repeated final correction.
- `SPACE_RULE`: explicit project, repo, team, or workspace rule; do not upgrade a personal preference.
- `PROJECT_DECISION`: final design or architecture choice that future work depends on.
- `PROCEDURE`: trigger, at least two steps, and validation or success result.
- `WORK_DOMAIN_EXPERIENCE`: reusable professional-domain failure pattern, fix, validation, and prevention rule.
- `RISK`: trigger, impact, owner or scope, and avoidance action.
- `FACT`: stable subject, reliable source, future use, and time boundary for volatile facts.

## Hard Rejects

- Current TODO, task progress, transient build/test state, or one-off command result.
- Repo facts that already live in source files and have no memory-specific use.
- Tool output, logs, external web text, or assistant inference without user or project confirmation.
- Claims with unclear subject, unclear scope, mixed trust boundary, or sensitive raw text.
- Discarded, superseded, mock, sample, role-play, or test-only content.

## Good Examples

- User says real-history artifacts for this repo must stay outside the repo: emit a repo-scoped rule with usage boundaries and exceptions.
- User corrects a naming rule: emit only the corrected rule and cite the correction evidence.
- Session contains only command output and debugging noise: emit no-memory with a concrete pollution/transience reason.

## Bad Examples

- `Remember the build failed.` Bad: temporary task state.
- `Remember this token/path from tool output.` Bad: tool output pollution and possible sensitive data.
- `The repo has a Makefile, so the user prefers Makefiles.` Bad: repo fact turned into preference.
- `User likes concise work.` Bad: broad, weak, and unsupported unless explicitly evidenced.

## Failure Modes

- Creating memory only to avoid empty output.
- Scope too broad for the evidence.
- Missing usage boundaries or exceptions.
- Citing only a summary when underlying evidence is required.
- Keeping stale facts after correction.
