# EliteForge Memory Dreaming Prompt Quality Foundation Brainstorm

## Context

Tron already has the repo-local `eliteforge-memory-dreaming` script foundation under `tron-skills/eliteforge-memory-dreaming`. The foundation can import Codex sessions, create DreamRun tasks, expose `task-detail`, validate task output from `tempOutputRef`, promote formal artifacts, and finalize deterministic reports.

The next task is not only to write the skill prompt and task prompts. The original IntentForge documents are preliminary design inputs, not proof that the resulting skill can extract and govern memory well. This feature must make memory quality testable.

Primary source documents:

- `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/memory-skill-ecosystem-design.md`
- `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/memory-dreaming-workflow-design.md`
- `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/memory-dreaming-task-prompt-contract-design.md`

Current repository context:

- Existing feature workspace: `docs/features/eliteforge-memory-dreaming-script-foundation/`
- Current script entry: `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`
- Current task implementation: `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib/tasks.py`
- Current schema implementation: `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib/schemas.py`
- Current test entry: `make skill-memory-dreaming-test`

## Problem

The existing script proves task orchestration, not memory quality. The current minimal schemas accept broad strings such as `summary`, `notes`, and `points[].text`. That is enough to test state transitions, but not enough to evaluate whether an agent:

- extracts durable memories instead of transient task progress;
- preserves evidence and source boundaries;
- separates user preference, project rule, agent experience, risk, procedure, and fact;
- avoids writing tool output, mock data, assistant guesses, external web material, or secrets as durable memory;
- handles REM decisions such as candidate creation, candidate reinforcement, ACTIVE equivalent reinforcement, conflict escalation, rejected-memory recheck, and idempotent no-write;
- handles Deep decisions such as duplicate candidate consolidation, conflict review, stale memory review, structure-health suggestions, and authorized no-op.

Without stronger schemas and golden evaluation cases, the project could ship polished prompt text that still produces low-quality or polluted memory artifacts.

## Selected Approach

Build a prompt quality foundation with three layers:

1. Prompt layer: concise `SKILL.md` orchestration guidance plus detailed task prompts in `references/`.
2. Script contract layer: `task-detail` returns the task prompt, prompt version, and task requirements; task outputs use richer schemas that expose quality-relevant fields.
3. Evaluation layer: deterministic golden fixtures and a quality evaluator that scores formal artifacts against expected positive, negative, and governance outcomes.

This keeps prompt content versioned with the skill while making quality measurable through `make skill-memory-dreaming-test`.

## Alternatives Considered

### Alternative A: Documentation-only prompt update

Update `SKILL.md` and references without changing script output or schemas.

This is insufficient because the script would still only validate broad text fields. It would not catch pollution, missing evidence, wrong scope, weak confidence, or bad governance decisions.

### Alternative B: Prompt references plus task-detail prompt output only

Add prompt files and return them from `task-detail`, but keep current minimal schemas.

This improves agent usability but still leaves evaluation weak. Tests could assert that prompts exist, not that outputs carry the structure needed to judge memory quality.

### Alternative C: Prompt quality foundation

Add prompt references, return prompts from `task-detail`, upgrade task output schemas, and add golden quality evaluation cases.

This is the recommended approach. It is larger, but it matches the true requirement: high-quality memory extraction and organization must be testable, not assumed.

## Design

### Skill Prompt Structure

`SKILL.md` should stay concise and authoritative. It should cover:

- when to use the skill;
- artifact root rules: explicit `--artifact-root` first, `.intentforge-dreaming/` for normal use;
- task command loop: `task-list -> task-detail -> task-start -> write tempOutputRef -> task-complete`;
- mandatory quality posture: evidence-first, no pollution, no long-term memory from transient task state;
- current V1 boundary: Codex provider only, script-only REM/Deep, no real Memory MCP calls;
- reference navigation for detailed prompts and evaluation rules.

Detailed prompt content should live under `tron-skills/eliteforge-memory-dreaming/references/`:

- `task-prompt-shared.md`
- `segment-summary-prompt.md`
- `session-summary-prompt.md`
- `light-memory-points-prompt.md`
- `rem-govern-memory-point-prompt.md`
- `deep-memory-governance-prompt.md`
- `quality-evaluation.md`

### Script Prompt Contract

`task-detail` should continue returning existing fields:

- `inputRefs`
- `preview`
- `schema`
- `tempOutputRef`

It should add:

- `promptVersion`
- `taskPrompt`
- `requirements`

`taskPrompt` is the concrete prompt text for the task type. It may be built from shared rules plus task-specific instructions. `requirements` is a compact structured summary that agents and tests can inspect without parsing Markdown.

The script should load prompt files from the skill `references/` directory at runtime. If a prompt file is missing, `task-detail` should fail with a typed error rather than returning an empty or generic prompt.

### Traceability Contract

Memory dreaming must be explainable after the fact. Every accepted memory point and every governance decision must trace back to the original Codex session material, the prompt version that produced it, and the task attempt that promoted it.

Each formal task output should include a top-level provenance envelope:

```json
{
  "dreamRunId": "20260528T120000Z-codex-abc123",
  "taskId": "task-0004",
  "taskType": "LIGHT_MEMORY_POINTS",
  "taskAttempt": 1,
  "promptVersion": "memory-dreaming.prompts.v2",
  "promptHash": "sha256:...",
  "inputRefs": [
    {"kind": "SESSION_SUMMARY", "ref": "sessions/<session-key>/session-summary.json", "sha256": "..."}
  ],
  "outputHash": "sha256:..."
}
```

Every semantic item should carry item-level `evidenceRefs[]`. Paths alone are not enough. A shared `EvidenceRef` object should include:

- `sessionKey`
- `segmentId`
- `eventIds` or `eventOrdinalRange`
- `sourceRole`
- `trustBoundary`
- `timestamp`
- `sourceRef`
- `sourceHash`
- optional `contentSha256`

The required trace chain is:

```text
Memory point or governance decision
-> pointId / decisionId
-> taskId + dreamRunId + promptVersion + promptHash
-> inputRefs
-> session summary evidence index
-> segmentId + event ids / event ordinals
-> snapshot.events[]
-> sourceRef + sourceHash from Codex JSONL
```

Current importer data is close but incomplete. Follow-up implementation must add enough event, segment, and task metadata so the chain can be resolved mechanically by the evaluator.

### Schema Direction

The current minimal schema should be upgraded enough to support quality evaluation. It should not become a full Memory Core schema, but every semantic claim must be inspectable. Because this is not backward-compatible with the existing minimal `*.v1` payloads, the richer artifacts should use new schema versions such as `memory-dreaming.light-memory-points.v2`.

Recommended V1 task output shape:

`SEGMENT_SUMMARY`:

- `schemaVersion`
- `sessionKey`
- `segmentId`
- `summary`
- `facts[]`
- `decisions[]`
- `risks[]`
- `discardedFindings[]`
- `memorySignals[]`

`SESSION_SUMMARY`:

- `schemaVersion`
- `sessionKey`
- `summary`
- `finalOutcomes[]`
- `finalDecisions[]`
- `stableFacts[]`
- `discardedIntermediateFindings[]`
- `memorySignalOverview[]`

`LIGHT_MEMORY_POINTS`:

- `schemaVersion`
- `sessionKey`
- `provenance`
- `points[]`
- each point includes `pointId`, `type`, `subject`, `claim`, `scopeHint`, `evidenceRefs`, `sourceRole`, `trustBoundary`, `confidence`, `importanceHint`, `memoryValueScore`, and `reason`
- `rejectedSignals[]` may record high-risk rejected signals for evaluation, without forcing downstream REM tasks
- `noPointReason` when `points` is empty

`REM_GOVERN_MEMORY_POINT`:

- `schemaVersion`
- `sessionKey`
- `provenance`
- `action`
- `memoryMcpCalls`
- `decisions[]`
- each decision includes `pointId`, `decision`, `targetScopeType`, `subject`, `matchedCards`, `evidenceRefs`, `idempotencyKey`, `memoryValueScore`, `requiresDeepReview`, and `reason`

`DEEP_MEMORY_GOVERNANCE`:

- `schemaVersion`
- `provenance`
- `action`
- `memoryMcpCalls`
- `sessionCount`
- `decisions[]`
- each decision includes `decision`, `target`, `affectedRefs`, `evidenceRefs`, `requiresAuthorization`, `riskLevel`, and `reason`
- `noOpReason` when no action is taken

Current V1 remains script-only:

- `REM_GOVERN_MEMORY_POINT.memoryMcpCalls` must be `0`.
- `DEEP_MEMORY_GOVERNANCE.memoryMcpCalls` must be `0`.
- `action` remains `DRY_RUN` or `NO_OP`.
- Prompts may describe future MCP semantics, but current executable instructions must not tell the agent to call Memory MCP.

### REM Task Granularity

The original task type is named `REM_GOVERN_MEMORY_POINT`, but the current script creates one session-level REM task after `LIGHT_MEMORY_POINTS`. That is too coarse for high-quality governance.

The recommended implementation is one REM task per accepted Light point:

- `LIGHT_MEMORY_POINTS.points[]` produces zero or more downstream REM tasks.
- If `points` is empty, no REM task is created for that session.
- Each REM task references one `pointId`, the Light artifact, and relevant session summary evidence.
- REM output remains per point, making idempotency, candidate reinforcement, conflict detection, and traceability concrete.

Deep still remains global for the run and starts after all point-level REM tasks are completed or skipped by absence of points.

### Quality Evaluation

Add deterministic fixtures and an evaluator for formal task artifacts. The evaluator should not rely on a model call. It should inspect structured outputs written by an agent and compare them with expected outcomes.

The evaluator should be manifest-driven. Each fixture scenario should include a semantic oracle:

```text
quality-fixtures/<scenario-id>/
  codex-home/sessions/*.jsonl
  mcp-state.json
  semantic-manifest.json
  golden/
    segment-summary.json
    session-summary.json
    light-memory-points.json
    rem-governance.json
    deep-governance.json
  mutants/
    schema-valid-wrong-scope.json
    schema-valid-polluted-tool-output.json
    schema-valid-missing-evidence.json
```

`semantic-manifest.json` should define expected accepted points, forbidden signals, expected rejected signals, expected REM/Deep decisions, allowed evidence refs, temporal expectations, and script-only authorization policy. Semantic matching should use canonical keys and explicit aliases, not LLM judgment or loose string similarity.

Evaluation dimensions:

- precision: extracted memories must be high-value, durable, and not transient;
- recall: required golden memories from fixtures must appear;
- pollution rate: forbidden signals must not appear as accepted points;
- evidence integrity: every accepted point and governance decision must reference input material;
- source-boundary integrity: user, assistant, tool, system, external web, and MCP output must not be confused;
- scope/type accuracy: memory type and scope hint must match the fixture expectation;
- governance decision accuracy: REM/Deep decisions must match expected candidate, reinforcement, conflict, no-write, or no-op outcomes;
- authorization safety: Deep must not claim high-impact writes in script-only mode.
- temporal and provenance correctness: relative time such as "today", "yesterday", and "recently" must be interpreted from source session timestamps, and outputs must preserve source refs instead of overwriting facts blindly.

Hard-fail conditions:

- any forbidden signal appears as an accepted point;
- any accepted point or governance decision lacks resolvable evidence;
- evidence source role or trust boundary does not match the manifest;
- `memoryMcpCalls != 0` in script-only V1;
- Deep claims unauthorized writes;
- REM or Deep emits decisions for nonexistent points or omits required decisions;
- relative time is resolved from the runtime clock instead of source timestamps.

Report metrics:

- precision, with safety fixtures requiring no false positives;
- recall for required golden memories;
- pollution rate, which must be zero;
- evidence integrity, which must be complete for accepted points and decisions;
- scope/type accuracy;
- governance decision accuracy;
- idempotency accuracy;
- run-to-run stability.

### Golden Fixture Categories

The evaluation suite should include at least these scenario groups:

- User preference: explicit long-term output or collaboration preference must be extracted.
- Project rule: AGENTS/project-level instruction must be treated as `SPACE_RULE`, not personal preference.
- Project decision: a final architecture or workflow choice must be extracted, while abandoned alternatives are rejected.
- Agent experience: a repeated or verified tool failure pattern must be extracted only when it has a reusable trigger and remedy.
- Risk: a rule that prevents memory pollution, unsafe writes, or destructive action must be extracted.
- Negative transient state: current task list, temporary progress, one-off command output, and ordinary file edits must not become memories.
- Negative source pollution: tool output, external web content, assistant speculation, mock data, and tests must not become user or project facts.
- Sensitive content: secrets or private identifiers must be excluded or recorded only as redacted governance facts.
- Temporal provenance: relative time claims must resolve against source event time, stale technical facts must carry review or expiry context, and conflicting later evidence must be treated as lifecycle evolution instead of silent replacement.
- REM candidate creation: a valid new point should produce `CREATE_CANDIDATE` in dry-run form.
- REM candidate reinforcement: independent evidence for an equivalent candidate should produce reinforcement, not duplicate creation.
- REM ACTIVE equivalent reinforcement: an equivalent active memory should be recognized as reinforcement only; no status downgrade.
- REM conflict: a conflicting point should require Deep review.
- REM idempotency: same source/evidence should become `NO_WRITE`.
- Deep no-op: no sessions or no maintenance target should produce an explicit, reasoned no-op.
- Deep governance plan: duplicate, stale, conflicting, or structurally unhealthy memory sets should produce review suggestions without unauthorized writes.

### Testing Strategy

Automated tests should cover:

- `task-detail` includes prompt metadata and task-specific requirements for all task types.
- Missing prompt reference files fail clearly.
- Rich schema validation rejects missing quality fields.
- Golden outputs pass the evaluator.
- Polluted outputs fail the evaluator.
- Current minimal legacy outputs either fail after schema upgrade or are migrated only where safe.
- REM/Deep outputs with `memoryMcpCalls != 0` fail in script-only mode.
- Makefile target remains the primary validation entry.

Manual or agent-assisted smoke should cover:

- Run `init-run` on deterministic fixture Codex sessions.
- Use the returned task prompts to produce real task outputs.
- Run the quality evaluator.
- Record whether the prompt produced high-quality extraction without hand-editing the artifacts.

This manual smoke is important because deterministic tests can validate the evaluator and golden contracts, but only an actual agent run can validate that the prompt text reliably guides generation.

### Real Codex Data Live Smoke

The skill must have a testing method for real local Codex data, but this method must not become a default CI gate and must not commit private artifacts.

Use a two-set live smoke:

1. Locked regression sample: 8-12 sessions selected by deterministic bucket rules before seeing prompt outputs.
2. Fresh holdout sample: another 8-12 sessions selected after prompt/schema changes to detect overfitting.

Suggested buckets:

- recent sessions;
- older sessions;
- archived sessions;
- large sessions;
- small sessions;
- project-related sessions;
- malformed or edge-case sessions when available.

Do not point the live smoke directly at the full `~/.codex` corpus. Build a temporary sampled Codex home with symlinks to selected JSONL files, then run:

```bash
python3 tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py init-run \
  --codex-home <sample-codex-home> \
  --artifact-root <repo-outside-artifact-root> \
  --provider codex
```

Live smoke pass criteria:

- no artifact is written under the repository;
- task outputs pass schema and evaluator checks;
- accepted memories have complete evidence refs;
- hard-exclusion pollution is zero;
- script-only REM/Deep keeps `memoryMcpCalls=0`;
- repeated runs over the same sample produce stable canonical memory keys;
- fresh holdout sample satisfies the same safety gates;
- final committed evidence contains aggregate metrics only.

Private local evidence may include salted sample IDs, bucket counts, command summaries, evaluator metrics, stability tables, and cleanup confirmation. Committed reports must not include source paths, session IDs, source hashes, previews, message text, or distilled private facts.

### Security And Retention

This feature must harden the script before running meaningful live smoke on real Codex data.

Required safeguards:

- Add `.intentforge-dreaming/` and memory dreaming temp output patterns to `.gitignore`.
- Make live smoke require an explicit repo-external `--artifact-root`.
- Treat `segments/*.material.md` as sensitive because it contains real message text.
- Redact or disable `task-detail.preview` for live smoke unless explicitly requested.
- Store relative source refs rather than public absolute paths where possible.
- Use private permissions for temp roots and temp output files.
- Delete or purge temp outputs after successful promotion.
- Provide a cleanup path for `/tmp/eliteforge-memory-dreaming`.
- Never commit live `runs/`, `sessions/`, temp outputs, command logs containing previews, or real-data quality reports with private facts.

### Out Of Scope

- Real Memory MCP read/write integration.
- Automatic candidate or ACTIVE Memory Core persistence.
- Backend MemoryDreamJob scheduling.
- Frontend UI.
- Multi-provider session import beyond Codex.
- Claiming prompt quality from schema-only tests.

## Risks And Mitigations

| Risk | Mitigation |
| --- | --- |
| Prompts pass schema but extract low-value memories | Use golden quality evaluator with positive and negative cases. |
| Deterministic tests cannot prove model behavior | Add manual agent-assisted smoke flow and make evaluator inspect generated artifacts. |
| Schema becomes too complex for V1 | Keep fields focused on quality evidence and governance decisions, not full Memory Core persistence. |
| Prompt files and script prompts drift | Script loads prompt references directly and tests assert task-specific prompt content. |
| REM/Deep prompt overpromises MCP behavior | Current prompt requirements explicitly require script-only dry-run and `memoryMcpCalls=0`. |
| Existing tests break due schema upgrade | Update tests as part of the feature and keep the Makefile target as the single validation entry. |
| Relative time is misread using the current runtime date | Prompts and evaluator require source timestamp based normalization or explicit uncertainty. |
| Real Codex smoke leaks private data | Use symlink sample homes, repo-external artifact roots, redacted reports, gitignore guards, and cleanup checks. |
| Memory artifacts cannot be audited later | Require provenance envelopes and shared `EvidenceRef` objects in all semantic outputs. |

## Acceptance Direction

The feature should be considered complete only when:

- skill prompt and task prompt references exist and are loaded by `task-detail`;
- output schemas expose the fields needed to evaluate memory quality and provenance;
- Light can produce an explicitly reasoned empty result without creating garbage memories;
- point-level REM tasks preserve per-point idempotency and traceability;
- deterministic tests cover all prompt and schema contracts;
- manifest-driven golden quality evaluator catches both good extraction and schema-valid pollution cases;
- real Codex live smoke method exists and records private local evidence without committing private artifacts;
- `.intentforge-dreaming/`, temp outputs, and live smoke artifacts are protected from accidental commit;
- `make skill-memory-dreaming-test` passes;
- `skill-creator` validation passes;
- feature docs record AC rows for prompt availability, schema quality, evaluator coverage, and script-only REM/Deep safety.
