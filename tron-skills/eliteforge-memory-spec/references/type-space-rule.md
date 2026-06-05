# Type: SPACE_RULE

Use `SPACE_RULE` for a durable rule, norm, or constraint that applies to a shared workspace, team, project space, or repository collaboration scope.

Do not use it for a single user's private preference, an agent-only lesson, or a file fact that is already maintained in repository documentation.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Rule: <ruleSubject> must/should/must not <normative behavior>.

Authority:
- <sourceAuthority and why it can set the rule>

Effective scope:
- <teams, repos, branches, apps, environments, or workflows covered>

Applies when:
- <situations where future agents must follow the rule>

Exceptions:
- <known exceptions or confirmation requirements>

Use by:
- <how the rule changes planning, editing, testing, deployment, or communication>

Evidence:
- <brief evidence summary that points to source_refs>
```

The first line must be normative. Avoid vague policy labels without an action.

## metadata.typeSpecific

Required fields:

- `ruleOwner`: team, workspace, governance body, or source that owns the rule.
- `ruleSubject`: artifact, workflow, behavior, system, or boundary controlled by the rule.
- `normativeLevel`: `must`, `should`, `must_not`, or a similarly explicit level.
- `sourceAuthority`: why the source can define this rule.
- `exceptions`: known exceptions, confirmation path, or "none known".
- `effectiveScope`: where the rule applies.

## semantic_key Pattern

Use:

```text
space-rule:<scope-code>:<rule-subject-slug>:<normative-level-slug>:<behavior-slug>
```

Rules:

- Use the owning space scope code.
- Include the controlled subject, not only the rule category.
- Change the key only when the rule subject or durable behavior changes.

## source_refs Requirement

`source_refs` must include the authoritative source: workspace instruction, approved project doc, governance decision, policy ticket, or explicit owner confirmation. Tool output alone is not enough authority for a `SPACE_RULE`.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "SPACE_RULE"
title: "Architecture changes require approved decision evidence"
semantic_key: "space-rule:product-platform:architecture-change:must:cite-approved-decision"
source_refs:
  - type: "governance_policy"
    id: "architecture-review-policy"
    locator: "decision authority"
    observed_at: "2026-06-03"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Architecture change decision authority"
  purpose: "prevent-unapproved-architecture-changes"
  typeSpecific:
    ruleOwner: "Product platform governance"
    ruleSubject: "architecture-changing implementation work"
    normativeLevel: "must"
    sourceAuthority: "Approved workspace governance policy"
    exceptions: "Emergency fixes may proceed only with owner confirmation and a follow-up decision record."
    effectiveScope: "Shared product platform architecture and integration work"
text: |
  Rule: Architecture-changing implementation work must cite an approved decision source before execution.

  Authority:
  - The workspace governance policy defines who can approve architecture changes.

  Effective scope:
  - Shared product platform architecture, integration boundaries, and cross-team technical decisions.

  Applies when:
  - Planning or implementing a change that alters system boundaries, shared contracts, or integration behavior.

  Exceptions:
  - Emergency fixes may proceed only with owner confirmation and a follow-up decision record.

  Use by:
  - Find the approved decision source before changing shared architecture.

  Evidence:
  - The referenced governance policy records the approval authority.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "SPACE_RULE"
title: "Follow strict rules"
semantic_key: "rule:strict"
source_refs:
  - type: "chat"
    id: "brainstorm"
metadata:
  typeSpecific:
    ruleOwner: "unknown"
    ruleSubject: "all work"
    normativeLevel: "maybe"
    sourceAuthority: "someone suggested it"
    exceptions: ""
    effectiveScope: "everything"
text: |
  Follow the strict process because it seems better.
```

Problems: weak authority, normative level is vague, scope is overbroad, exceptions are missing, and the text does not state a durable workspace rule.
