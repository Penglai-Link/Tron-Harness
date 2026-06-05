# Type: USER_PREFERENCE

Use `USER_PREFERENCE` for a durable preference owned by a user scope. It describes how the user wants future collaboration, output, planning, communication, or execution to behave.

Do not use it for one-off instructions, inferred taste, external profile data, project rules, or agent execution lessons.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Preference: <preferenceOwner> prefers <behavior> for <preferenceTarget>.

Applies when:
- <future situation where the preference should influence output or action>

Do not apply when:
- <exception, one-off override, or scope boundary>

Use by:
- <how a future agent should change wording, plan, tool choice, or execution>

Evidence:
- <brief evidence summary that points to source_refs, not raw transcript text>
```

Keep the first sentence directly usable. Do not hide the durable behavior in a story about when the preference was discovered.

## metadata.typeSpecific

Required fields:

- `preferenceOwner`: stable owner or persona the preference belongs to.
- `preferenceTarget`: output, workflow, collaboration area, tool, or content class affected.
- `behavior`: the expected future behavior.
- `strength`: strength such as `explicit`, `strong`, `soft`, or `situational`.
- `exceptions`: known boundaries, overrides, or "none known".

## semantic_key Pattern

Use:

```text
user-preference:<scope-code>:<owner-slug>:<target-slug>:<behavior-slug>
```

Rules:

- Use the owning user scope code, not a display name alone.
- Keep `target-slug` stable across wording changes.
- Do not add dates unless the preference is explicitly time-bound.

## source_refs Requirement

`source_refs` must include the user statement, approved profile source, or governance event that authorized the preference. Inferred preferences require repeated strong evidence and should usually remain candidate or review-only.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "USER_PREFERENCE"
title: "User prefers concise implementation summaries"
semantic_key: "user-preference:usr-admin:admin:delivery-summary:concise-implementation"
source_refs:
  - type: "chat"
    id: "session-2026-06-03-a"
    locator: "turn-18"
    observed_at: "2026-06-03"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "USR-ADMIN"
  purpose: "delivery-summary-style"
  typeSpecific:
    preferenceOwner: "USR-ADMIN"
    preferenceTarget: "delivery summaries after code changes"
    behavior: "summarize completed changes and verification concisely"
    strength: "explicit"
    exceptions: "Provide more detail when the user asks for a review, RCA, or technical explanation."
text: |
  Preference: USR-ADMIN prefers concise implementation summaries after code changes.

  Applies when:
  - Reporting completed file edits, validation, and residual risks.

  Do not apply when:
  - The user asks for a detailed review, root-cause analysis, or design explanation.

  Use by:
  - Lead with changed files and verification, then list only material risks.

  Evidence:
  - The user explicitly requested shorter delivery summaries in the referenced chat turn.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "USER_PREFERENCE"
title: "User likes short answers"
semantic_key: "preference:short"
source_refs: []
metadata:
  typeSpecific:
    preferenceOwner: "the user"
    preferenceTarget: "everything"
    behavior: "be short"
    strength: "inferred"
    exceptions: ""
text: |
  The user probably likes short answers because they said "shorter" once.
```

Problems: no source refs, unstable subject, overgeneralized target, weak inference, empty exceptions, and no apply or non-apply boundary.
