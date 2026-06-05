---
name: eliteforge-memory-dreaming
description: Use when running batch memory dreaming with the local memory-dream command over an approved historical scope, including Segment, Session, Light, REM, Deep, and final report tasks.
metadata:
  version: 1.2.0
---

# EliteForge Memory Dreaming

Run CLI-governed semantic review through `memory-dream`. The CLI owns run state, task materials, validation, artifacts, provenance, promotion, and reports. The agent owns interpretation, evidence boundaries, candidate quality, rejection reasons, and governance reasoning. Passing schema is necessary but not sufficient for a good result.

## Gate

- Require `memory-dream` in `PATH`. If missing, stop and report it.
- Before the first run, read `memory-dream --help`; before unfamiliar subcommands, read that subcommand's help.
- Treat command help and subcommand help as the command contract.
- Treat `task-background.data.task.outputSchema` as the only submit JSON contract.
- Do not cache static schemas, infer hidden behavior, or use this skill as a second CLI README.

## Workflow

1. Use an approved run id from the user, or run `init-run` from an approved historical scope.
2. `task-list` to choose the next task.
3. `task-background` to read task metadata, materials, first pages, and `outputSchema`.
4. For each `data.materials[]` entry, ensure pages `1..pageCount` are read. `firstPage` may satisfy page 1; use `task-material` for any page not already present.
5. `task-start` only when ready to work on that task.
6. Produce one JSON object that satisfies the current `outputSchema`, then submit it through `task-submit` via stdin or `--input-file`.
7. Use `status` to determine whether more tasks remain or `data.canFinalize=true`.
8. Run `finalize-run` only when `status.data.canFinalize=true`.

Before submitting, read the quality reference that matches the task type:
   - `SEGMENT_SUMMARY` -> `references/quality-segment.md`
   - `SESSION_SUMMARY` -> `references/quality-session.md`
   - `LIGHT_MEMORY_POINTS` -> `references/quality-light.md`
   - `REM_GOVERN_MEMORY_POINT` -> `references/quality-rem.md`
   - `DEEP_MEMORY_GOVERNANCE` -> `references/quality-deep.md`

## Materials

- Use only CLI output: `task-background` data, `firstPage`, and `task-material` pages.
- Do not inspect hidden workspace files, internal refs, source session files, artifact output files, run state, or report files.
- If material is missing or ambiguous, ask the CLI for the documented page or task data; do not reconstruct it from filesystem paths.

## Output

- Do not write artifacts directly.
- Submit exactly one JSON object with `task-submit` via stdin or `--input-file`.
- Omit provenance fields; if the current `outputSchema` appears to require provenance, stop and report the CLI contract conflict.
- The CLI owns storage, promotion, provenance, artifacts, and final reports.

## Semantic Bar

- Prefer no memory over weak memory.
- Preserve evidence boundary, latest correction, source role, trust boundary, scope, subject, purpose, and rejection reasons.
- Do not promote tool output, assistant guesses, task status, repo-file facts, debug noise, stale facts, or hidden file contents into memory.
- Use `eliteforge-memory-spec` for memory quality and governance, but current semantic scopes are only `USER`, `WORK_DOMAIN`, and `SPACE`; emit the spelling required by the current `outputSchema`.
- Do not emit old `AGENT` scope. Use `WORK_DOMAIN` only for a recognizable professional domain with reusable value beyond one project or task; domain names are not scope values by themselves.

## Memory Tool

- Keep Memory Tool use inside the current task's evidence boundary and the active `eliteforge-memory-spec` rules.
- Treat `outputSchema` as the task-submit JSON format contract.
- Never claim a real memory operation, identifier, status change, deletion, or saved record without an actual Memory Tool result.

## Recovery

- If `task-submit` rejects output, fix the JSON object against the current `outputSchema` and submit again.
- Fail a task only when no valid output can be produced.
- Do not invent skip, checkpoint, or direct-write commands.
