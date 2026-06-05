---
name: eliteforge-memory-spec
description: >
  MUST be loaded at the beginning of every session. After loading it, the agent
  MUST fan out memory_search with mode=CONTEXT_PACK for USER, SPACE, and
  WORK_DOMAIN before the first user-visible response or repository action,
  including greetings, acknowledgements, status replies, and clarification
  questions. Reading this skill is not memory retrieval.
  This spec governs memory retrieval, creation, review, update, deletion,
  deduplication, conflict handling, scope, memory_type, language, evidence,
  status, injectable, and proof rules.
metadata:
  version: 1.2.1
  load_policy: session_start_required
  context_bootstrap_policy: context_pack_three_scope_fanout_before_first_response
  priority: critical
---

# EliteForge Memory Spec

Define persisted memory content format, type-specific text structure, quality gates, and governance rules.

This skill is not an MCP invocation schema. When calling memory tools, follow the current tool schema/help. Use the record envelope below only to author and review memory content before mapping it to tool parameters.

## Mandatory Fan-Out Bootstrap

After loading this skill, fan out `memory_search(mode=CONTEXT_PACK)` for `scope=USER`, `scope=SPACE`, and `scope=WORK_DOMAIN` before the first user-visible response or repository action. Reading this skill is not reading long-term memory; only current memory tool results prove retrieval happened.

Greetings, acknowledgements, status replies, and clarification questions are user-visible responses. Do not answer them before the fan-out bootstrap.

Always attempt all three bootstrap scopes:

1. Request `USER` context for user preferences, collaboration rules, and personal constraints.
2. Request `SPACE` context for workspace, repository, project, and team constraints.
3. Request `WORK_DOMAIN` context for reusable professional-domain procedures, risks, and lessons.

Do not skip any bootstrap scope because the user message is a greeting, acknowledgement, status question, unclear task, unclear workspace, or unclear domain. Relevance controls whether retrieved memory affects behavior; it does not control whether retrieval is attempted.

When `scope_code` is unknown, first use the scope-only call supported by the current tool schema, such as `scope=USER`, `scope=SPACE`, or `scope=WORK_DOMAIN` without `scope_code`. Do not replace a missing or failing scoped bootstrap with an unscoped `CONTEXT_PACK` call and then claim the three-domain bootstrap is complete.

If one bootstrap scope fails, record only that scope as degraded and proceed with no durable-memory assumptions for that scope until retrieval succeeds. Retry that scope when the task depends on prior preferences, project rules, procedures, risks, or decisions.

Before the first response or action, confirm internally:

- This skill was loaded and the current memory tool was called with `mode=CONTEXT_PACK` for `USER`, `SPACE`, and `WORK_DOMAIN`.
- Each of `USER`, `SPACE`, and `WORK_DOMAIN` has current tool evidence: loaded results, an empty result, or a disclosed failure/degraded result for that specific scope.
- Full bootstrap complete may be claimed only when all three scopes have current tool evidence.
- Only current `ACTIVE`/`APPROVED` retrieval results can affect behavior.
- Obvious smoke, test, placeholder, low-quality, conflicting, or irrelevant memories must not affect behavior, even if returned as injectable. Do not treat such memories as a reason to skip `SPACE` retrieval.

## Boundary

- Use this skill for persisted long-term memory content, memory quality review, or converting memory extraction and governance signals into durable records.
- Do not use it for memory tool setup, authentication, retry syntax, or parameter lookup.
- Prefer no memory over weak, source-poor, transient, or ambiguous memory.
- Never persist raw transcripts, raw tool output, secrets, temporary task state, or assistant guesses as durable facts.

## Workflow

1. Identify the owning scope: `USER`, `WORK_DOMAIN`, or `SPACE`.
2. Determine `metadata.language` from the source language or explicit user request.
3. Identify the stable subject, purpose, scope boundary, and evidence source before writing.
4. Select exactly one memory type and read its reference:
   - `USER_PREFERENCE` -> `references/type-user-preference.md`
   - `SPACE_RULE` -> `references/type-space-rule.md`
   - `PROJECT_DECISION` -> `references/type-project-decision.md`
   - `PROCEDURE` -> `references/type-procedure.md`
   - `WORK_DOMAIN_EXPERIENCE` -> `references/type-work-domain-experience.md`
   - `RISK` -> `references/type-risk.md`
   - `FACT` -> `references/type-fact.md`
5. Build the persisted record with a stable `title`, exact `memory_type`, stable `semantic_key`, non-empty `source_refs`, `metadata.language`, `metadata.typeSpecific`, and normalized `text`.
6. Review the good and bad examples in the selected reference before saving or approving.

## Vocabulary

### Scope

- `USER`: memory governed by one user. Use for personal preferences, collaboration habits, constraints, and user-authorized personal or reference material.
- `WORK_DOMAIN`: memory governed by a reusable professional domain. Use it for knowledge that applies beyond the current project, repository, workspace, or one-off task, not for arbitrary lessons learned during an agent run.
- `SPACE`: memory governed by a workspace or project. Use for shared rules, project facts, decisions, procedures, and terminology.

Scope is the governance boundary, not always the subject. A `USER` memory can describe third-party reference material owned by that user; it must not silently become the user's identity.

For `WORK_DOMAIN`, choose the narrowest recognizable domain that remains reusable. Use a broad domain such as software development when the lesson applies across many development contexts. Use a subdomain such as frontend development, backend development, testing/QA, DevOps/release engineering, security/compliance, data analysis, product management, documentation governance, or project management when the lesson depends on that area. Use a technology-specific domain such as Java development or Python development only when the lesson depends on that language, framework, runtime, or ecosystem.

Do not turn domain names into `scope` values. The scope remains `WORK_DOMAIN`. Do not invent `scope_code` values; use `scope_code` only when the tool configuration, user instruction, or project convention provides a recognized work-domain code. Put the domain in `metadata.subject`, `metadata.purpose`, or `semantic_key` when useful; use `metadata.typeSpecific` only when the selected type reference or current schema explicitly defines a domain/category field. If no recognizable professional domain can be named, do not use `WORK_DOMAIN`.

### Memory Type

- `USER_PREFERENCE`: durable user preference or constraint about interaction, output, workflow, or working style.
- `SPACE_RULE`: shared workspace, project, team, or repository rule that future agents should follow.
- `WORK_DOMAIN_EXPERIENCE`: reusable lesson, failure mode, recovery action, validation strategy, or prevention rule that applies within an identifiable professional domain.
- `PROJECT_DECISION`: durable product, architecture, technical, naming, workflow, or delivery decision.
- `PROCEDURE`: reusable steps with trigger, prerequisites, success criteria, validation, and fallback.
- `RISK`: repeatable hazard, anti-pattern, warning, or safety boundary with avoidance action.
- `FACT`: stable fact or terminology with source authority, stability, and time boundary.

### Memory Status

- `ACTIVE`: may affect future behavior when approved and eligible for retrieval.
- `CANDIDATE`: useful but not approved to affect behavior.
- `REJECTED`: evaluated and rejected; keep only as governance history.
- `ARCHIVED`: obsolete or historical; keep for traceability, not behavior.
- `DISABLED`: unsafe, wrong, polluted, or misleading; must not affect behavior.

### Review Status

- `APPROVED`: review status meaning the memory is accepted for its current status.
- `PENDING_REVIEW`: review status meaning the memory needs human or later governance review.
- `REJECTED`: review status meaning the memory is not accepted as valid active memory.

### Injectable

`injectable=true` means scope baseline memory: facts or rules an agent must know before normal work in that owning scope.

- `USER`: language, response format, permission boundary, disliked behavior, validation expectation, secret-handling preference.
- `WORK_DOMAIN`: domain-wide execution constraint, tool safety rule, validation habit, or repeated failure prevention that transfers across projects in that domain.
- `SPACE`: hard rule, project boundary, accepted architecture decision, technical or API constraint, validation rule, safety or compliance rule, terminology.
- Otherwise pass `injectable=false` explicitly.

Do not set `injectable=true` merely because a memory is true, approved, or useful. When saving ordinary non-baseline memories, explicitly pass `injectable=false`; do not rely on tool defaults for `injectable`.

## Shared Record Envelope

Use this shared content model when authoring or reviewing a persisted memory. It is not an MCP invocation schema; map it to the current memory tool schema or help when saving:

```yaml
scope: "USER | WORK_DOMAIN | SPACE"
scope_code: "<owning scope code when known or required>"
memory_type: "<one of the seven allowed memory types>"
title: "<short stable retrieval title>"
text: "<type-specific Markdown text format>"
semantic_key: "<type-specific stable key>"
memory_status: "ACTIVE | CANDIDATE | REJECTED | ARCHIVED | DISABLED"
review_status: "APPROVED | PENDING_REVIEW | REJECTED"
injectable: "<true only for scope baseline memory>"
source_refs:
  - type: "<source type>"
    id: "<stable source id>"
    locator: "<optional source locator>"
    observed_at: "<date or timestamp when available>"
metadata:
  formatVersion: "memory-spec.v1"
  language: "<BCP-47 language tag, such as zh-CN or en-US>"
  subject: "<stable memory subject>"
  purpose: "<future use that justifies retrieval>"
  typeSpecific: {}
```

- `memory_type` must be one of the seven exact values listed above.
- `title` must be short, stable, and specific enough to choose from search results.
- `semantic_key` is required for new durable memories. Use lowercase ASCII segments separated by `:`. Use stable subject and purpose segments, not date-only keys or random ids.
- `source_refs` is required and must contain at least one durable evidence pointer. A source ref should identify source type, stable id, locator or range when available, and observed date or hash when available.
- `metadata.formatVersion` must be `memory-spec.v1`.
- `metadata.language` must identify the source or user-requested language with a BCP-47 tag such as `zh-CN` or `en-US`.
- `metadata.subject` must name the stable subject described by the memory.
- `metadata.purpose` must name the future use that makes the memory worth retrieving.
- `metadata.typeSpecific` is required and must contain the exact required fields from the selected type reference.
- Type references constrain type-specific `text`, `semantic_key`, `metadata.typeSpecific`, and minimum `source_refs` rules. Combine them with this shared envelope before saving.
- `text` is plain Markdown intended for retrieval and context injection. Do not use YAML front matter inside `text`; keep structured fields in metadata.
- `text` must state the durable claim first, then when to apply it, when not to apply it, how to use it, and what evidence supports it.

## Language Policy

- Skill files may be English; persisted human-readable memory content must follow `metadata.language`.
- For direct user-requested memories, set `metadata.language` from the user's request language and use it for `title`, `text`, `metadata.purpose`, human-readable `metadata.typeSpecific` values, human-readable `source_refs.locator` values, and evidence summaries.
- Do not translate simple user preferences, rules, facts, or instructions into English unless the user asks for English.
- Keep enum values, tool parameters, metadata keys, `memory_type`, statuses, and `semantic_key` in their required machine-readable form.
- For mixed-language sources, use the dominant user-facing language and preserve exact source phrases when wording matters.

## Memory Tool Use

- Use the current memory tool schema or help as the only parameter contract; this skill defines memory content and governance rules, not tool signatures.
- Treat the three-scope fan-out bootstrap `CONTEXT_PACK` gate as mandatory. Refresh `CONTEXT_PACK` when the active user, space, domain, or task scope changes; use `SEARCH` for follow-up lookup, duplicate checks, conflict checks, and governance.
- Before saving, search the same scope, subject, and purpose for duplicates and conflicts. Include non-active history when it could affect update, archive, disable, or rejection decisions.
- When updating or deleting by `memory_code`, do not also pass `scope` or `scope_code`.
- When using `scope_code`, pass `scope` in the same call.
- For new ordinary memories, pass `memory_status=ACTIVE`, `review_status=APPROVED`, and `injectable=false` unless the memory is explicitly a scope baseline memory.
- For candidate, rejected, archived, or disabled memories, keep `injectable=false`.
- Do not keep conflicting `ACTIVE` memories side by side in the same scope for the same subject and purpose. Update, disable, archive, reject, or ask for confirmation.
- Use `memory_delete` for soft deletion. Provide `memory_code`, a concrete `reason`, and `delete_mode` as `ARCHIVE` or `DISABLE`. Use `ARCHIVE` for obsolete or superseded records and `DISABLE` for unsafe, wrong, polluted, or misleading records.
- A memory retrieval claim is proven only by the current `memory_search` result. Do not claim long-term memory was loaded, unavailable, empty, scoped, or fully bootstrapped without current tool evidence for `USER`, `SPACE`, and `WORK_DOMAIN`.
- A memory write or governance action is proven only by the current tool result. Do not claim persistence, identifiers, status changes, or deletion without tool evidence.

## Text Quality

A persisted memory text is good when a future agent can decide whether and how to use it without reading the original transcript.

- Make one memory about one stable claim, preference, rule, decision, procedure, experience, risk, or fact.
- Write concrete subjects. Avoid drifting phrases such as "this project", "the user", "we", or "current agent" unless the scope and subject are explicit nearby.
- Include limits, exceptions, time boundaries, and review needs when they matter.
- Keep operational details reusable. Remove one-off commands, temporary filenames, transient errors, and unverified hypotheses.
- If the source is uncertain, conflicting, stale, or only inferred, do not persist it as `ACTIVE` durable memory.
- Reject task status, command output, file-change lists, transient errors, repository facts already maintained in files, assistant guesses, role-play content, test-only samples, and raw secrets.
