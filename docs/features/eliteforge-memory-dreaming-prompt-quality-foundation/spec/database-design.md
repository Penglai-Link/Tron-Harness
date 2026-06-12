# Database Decision

status: approved

No relational database or persistent backend schema is introduced. This feature defines file-based artifact schemas for the local dreaming runtime and quality fixtures.

# Existing Schema Impact

- Existing V1 task completion JSON remains a scaffold and is superseded by V2 task artifacts for formal prompt-quality execution.
- Existing run manifests, task files, segment materials, and final reports remain file-based.
- Existing fixture tests must be updated to the V2 structure where they exercise formal task completion.

# Proposed Schema

## Provenance Envelope

Every accepted task output must include:

- `dreamRunId`
- `taskId`
- `taskType`
- `taskAttempt`
- `promptVersion`
- `promptHash`
- `inputRefs`
- `outputHash`

## EvidenceRef

Every extracted memory point and governance decision must include resolvable evidence references:

- `sessionKey`
- `segmentId`
- `sourceRole`
- `trustBoundary`
- `sourceRef`
- `sourceHash`
- optional `eventIds`
- optional `eventOrdinalRange`
- optional `timestamp`
- optional `contentSha256`

## Light Memory Points

Light output includes:

- `points`: list of memory candidates
- `noPointReason`: required when `points` is empty
- `omittedSignals`: optional explanation for considered but rejected signals
- `memoryMcpCalls`: must be zero in V1

Each point includes stable memory category, durability, scope, confidence, normalized text, evidence refs, and governance hints.

## Summary Artifacts

Segment and session summaries include structured facts, decisions, constraints, unresolved questions, and evidence refs.

## REM And Deep Governance

REM output is per memory point and includes action decisions such as keep, merge, supersede, defer, or reject. Deep output is generated only when REM identifies consolidation work.

## Semantic Manifest

Fixtures define expected signals, forbidden signals, required evidence, governance decisions, and scoring thresholds.

# Relationships And Cardinality

- One `dreamRun` has many `sessions`.
- One `session` has many `segments`.
- One `segment` has many task inputs.
- One Light task may produce many memory points.
- One memory point has at least one evidence reference.
- One memory point creates zero or one REM task.
- One REM task creates zero or one Deep task.
- One semantic manifest covers one fixture scenario and may include many mutants.

# Migration Plan

- Add V2 validation alongside the current script foundation and route formal task completion through V2.
- Update synthetic tests and fixtures to V2.
- Keep legacy fixture helpers only where they support bootstrap tests.
- Treat existing runtime artifacts as disposable generated data; no migration is required.

# Query And Performance Plan

- File reads should remain bounded by explicit session selection, fixture roots, or `--max-sessions`.
- Live smoke uses sampled symlink Codex homes to avoid scanning the full real corpus.
- Hashing should be limited to selected source files and generated artifacts.
- Quality evaluation should operate on fixture directories and canonicalized JSON.

# Data Governance

- `.intentforge-dreaming/` and private live-smoke outputs must be ignored by Git.
- Raw segment materials are private runtime artifacts.
- `task-detail` must avoid leaking raw conversation text unless an explicit local opt-in flag is used.
- Temp directories must use restrictive permissions and support cleanup.
- Committed reports must contain only aggregate live-smoke metrics and no private paths, raw snippets, or source hashes.

# Test Data And Fixtures

Synthetic fixture bundles live under `tron-skills/eliteforge-memory-dreaming/quality-fixtures/`.

Each scenario may contain:

- `codex-home/sessions/*.jsonl`
- `mcp-state.json`
- `semantic-manifest.json`
- `golden/*.json`
- `mutants/*.json`

Fixtures must cover positive memories, no-memory sessions, stale or corrected facts, tool-output pollution, cross-session consolidation, and traceability failures.

# Validation Commands

- `make skill-memory-dreaming-test`
- `make skill-memory-dreaming-quality-test`
- `python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/eliteforge-memory-dreaming-prompt-quality-foundation --phase spec --complete-spec`
