# Type: PROJECT_DECISION

Use `PROJECT_DECISION` for a durable decision about architecture, API shape, workflow, feature scope, design direction, naming, data model, or delivery strategy.

Do not use it for current task status, an unapproved proposal, a personal preference, or a fact that could change without governance.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Decision: <decision>.

Rationale:
- <why this decision was made>

Affected areas:
- <systems, modules, docs, workflows, or teams affected>

Phase and time boundary:
- <phase, effective window, replacement condition, or review point>

Applies when:
- <future situations where the decision should guide work>

Do not apply when:
- <known exclusions, superseding decisions, or uncertainty>

Evidence:
- <brief evidence summary that points to source_refs>
```

The first line must be the decision itself, not the discussion history.

## metadata.typeSpecific

Required fields:

- `decision`: the durable decision in one sentence.
- `rationale`: the reason the decision was accepted.
- `phase`: lifecycle phase such as `prototype`, `first-version`, `production`, or `deprecated`.
- `temporalBoundary`: effective window, review condition, or "until superseded".
- `affectedAreas`: non-empty list of affected modules, interfaces, workflows, or docs.

## semantic_key Pattern

Use:

```text
project-decision:<scope-code>:<project-slug>:<decision-topic-slug>:<phase-slug>
```

Rules:

- Keep the topic stable even if wording changes.
- Include the project or product slug when the space contains multiple projects.
- Add a phase segment when the same topic may have prototype and production decisions.

## source_refs Requirement

`source_refs` must include an accepted decision source: approved spec, PRD, design review, issue decision, ADR, or explicit owner confirmation. A proposal-only discussion can create a candidate but not an approved decision memory.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "PROJECT_DECISION"
title: "Customer export audit metadata remains immutable"
semantic_key: "project-decision:customer-portal:export-audit-metadata:production"
source_refs:
  - type: "decision_record"
    id: "ADR-018-export-audit"
    locator: "accepted decision"
    observed_at: "2026-06-03"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Customer data export audit metadata"
  purpose: "preserve-export-auditability"
  typeSpecific:
    decision: "Customer export jobs store immutable audit metadata for requester, export time, filter summary, and approval source."
    rationale: "Auditability requires each export to retain provenance even if customer records later change."
    phase: "production"
    temporalBoundary: "Until a later accepted compliance decision supersedes this rule."
    affectedAreas:
      - "customer export workflow"
      - "audit logging"
      - "compliance reporting"
text: |
  Decision: Customer export jobs store immutable audit metadata for requester, export time, filter summary, and approval source.

  Rationale:
  - Auditability requires each export to retain provenance even if customer records later change.

  Affected areas:
  - Customer export workflow, audit logging, and compliance reporting.

  Phase and time boundary:
  - Production decision until a later accepted compliance decision supersedes it.

  Applies when:
  - Designing, reviewing, or changing customer data export behavior.

  Do not apply when:
  - A later accepted compliance decision defines a different audit model.

  Evidence:
  - The referenced decision record approves the export audit model.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "PROJECT_DECISION"
title: "Use simple design"
semantic_key: "decision:simple"
source_refs:
  - type: "chat"
    id: "brainstorm"
metadata:
  typeSpecific:
    decision: "Use simple design"
    rationale: "Someone mentioned it."
    phase: "unknown"
    temporalBoundary: ""
    affectedAreas: []
text: |
  We decided to keep it simple.
```

Problems: vague decision, no approved authority, missing temporal boundary, empty affected areas, and no apply or supersession guidance.
