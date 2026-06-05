# Session Quality

## Goal

Merge accepted segment outputs into a session-level memory analysis summary. Resolve repetition and corrections without creating final memory candidates.

## Rules

- Use accepted segment outputs as the source; reread source material only through CLI-provided task material pages when the task requires it.
- Merge duplicate signals without losing scope, subject, purpose, or evidence.
- Prefer the latest explicit correction over older statements.
- Preserve discarded findings and pollution reasons for Light.
- Keep evidence traceable back to segment and source refs.
- Separate durable signals from transient task execution.
- Do not invent final memory status.
- Do not create final memory points or decide final status.

## Output Focus

- `summary`: final session narrative, stripped of failed intermediate attempts.
- `keyPoints`: final outcomes, final decisions, stable facts, durable risks, and reusable procedures.
- `candidateSignals`: merged durable candidate signals with subject, scope clue, purpose, and evidence.
- `discardedIntermediateFindings`: corrected, failed, temporary, polluted, or superseded findings.
- `evidenceIndex`: map every stable claim to segment ids, source roles, trust boundaries, and refs.

## Good Examples

- Two segments repeat the same repo convention: merge into one stable session signal with both evidence refs.
- A later segment corrects an earlier setup command: keep the corrected value and note the old value as superseded.
- A segment discarded tool noise: carry that discarded finding forward so Light can return no-memory with a reason.

## Bad Examples

- `The user wants all future projects to follow this repo rule.` Bad: overbroad scope.
- `Assistant suspected a preference, so record it as stable.` Bad: assistant guesses are not user evidence.
- Dropping all discarded findings. Bad: Light loses rejection evidence.

## Failure Modes

- Losing cross-segment corrections.
- Turning session summary text into primary evidence.
- Over-compressing until evidence cannot be resolved.
- Treating transient task success or failure as durable memory.
- Upgrading repo-specific signals into global user preferences.
