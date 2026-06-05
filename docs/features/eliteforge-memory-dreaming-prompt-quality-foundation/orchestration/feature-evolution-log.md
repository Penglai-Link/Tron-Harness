# Summary

feature_slug: eliteforge-memory-dreaming-prompt-quality-foundation
status: reopened_for_skill_quality

# Change History

- Reopened prompt-quality foundation to correct the execution model: real quality acceptance requires a private Codex CLI session where the Agent uses the skill to perform memory dreaming, not a separate automation layer that substitutes for the Agent.
- Added real-agent skill quality, high-quality prompt constraints, no-MCP REM/Deep dry-run governance, and sanitized private evaluation evidence to the acceptance scope.
- Prompt-quality hardening expanded AC-MDPQ-01 and AC-MDPQ-04 evidence with no-memory and stale-correction fixtures. Scope remains inside the approved prompt-quality foundation.
- Added atomic source-analysis matrices for flow, quality scoring, governance, long-term memory behavior, and current implementation gaps so AC-MDPQ-08 through AC-MDPQ-12 can be implemented by adding detail from the original design instead of reducing it.
- Added `source-coverage-audit.md` and thickened the matrices to FLOW-055, QUALITY-085, GOV-056, and LTM-078 after read-only source coverage review found compressed original-design details.
- Recorded the user supplemental requirement that final skill-quality judgment must use local real Codex conversation data through a real Codex CLI Agent session; the current preparation stage records the testable protocol and does not run private history evaluation yet.
- Integrated AC-MDPQ-08/09 skill-quality prompt slice: SKILL.md now acts as a real Codex Agent entrypoint, references distinguish source channels and quality gates, prompts and task-detail requirements carry evidence/scope/pollution/noPointReason rules, and synthetic tests cover Light validation and evaluator thresholds.
- Integrated AC-MDPQ-10 governance dry-run slice: REM and Deep now require MCP-ready dry-run envelopes, read/write intents, proposed memory operations, authorization evidence, escalation reasons, Deep request coverage, and fake-execution rejection while keeping Memory MCP calls at zero.
- Prepared AC-MDPQ-11/12 private evaluation V1 prompt for user approval: the proposed run uses real `codex exec`, sampled local Codex history, repo-external `/tmp` artifacts, Agent-owned semantic dreaming, and sanitized aggregate-only review output.
- Completed AC-MDPQ-11/12 private evaluation after user approval: real `codex exec` sampled 8 local Codex sessions, completed 34 script-governed tasks, kept Memory MCP calls at zero, returned sanitized AC11/12 PASS evidence, and produced `reports/private-evaluation-report.md`.
- Reopened for AC-MDPQ-13..17 after quality review of the private run found thin memory content and weak traceability: added bounded segmentation, candidateSignals/coverageStats, claim-level EvidenceRef, richer Light usage fields, stricter evaluator mutants, and a stronger private evaluation rubric.

# Downstream Artifact Migration

None.

# Residual Risks

- Future quality changes should rerun the private Codex CLI evaluation or an approved equivalent private regression gate.
- The previous AC-MDPQ-11/12 private run predates AC-MDPQ-13..17, so final quality should not be claimed until a new approved real-history Codex CLI run evaluates the upgraded prompt/schema contract.
- Any blocker/high row in `source-coverage-audit.md` without implementation evidence or explicit user-approved out-of-scope status blocks completion.
