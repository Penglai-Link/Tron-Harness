# Type: FACT

Use `FACT` for a durable factual statement that future agents may rely on after checking source authority, stability, and time boundary.

Do not use it for preferences, rules, decisions, procedures, risks, assistant guesses, raw external material, or facts already better maintained in a live source of truth.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Fact: <factSubject> <durable factual statement>.

Source authority:
- <why the source can support this fact>

Stability and time boundary:
- <stable, time_sensitive, volatile, or historical; include effective or review boundary>

Use by:
- <how future agents should use this fact>

Do not use when:
- <conditions that make the fact stale, irrelevant, or unsafe>

Evidence:
- <brief evidence summary that points to source_refs>
```

The first line must be factual and bounded. Avoid unbounded claims about current state without dates or review conditions.

## metadata.typeSpecific

Required fields:

- `factSubject`: the entity, person, project, system, or artifact the fact describes.
- `sourceAuthority`: why the source can support the fact.
- `stability`: `stable`, `time_sensitive`, `volatile`, or `historical`.
- `timeBoundary`: effective date, observation date, review window, or "until superseded".
- `reviewAt`: required when `stability` is `volatile` or `time_sensitive`.

## semantic_key Pattern

Use:

```text
fact:<scope-code>:<fact-subject-slug>:<fact-name-slug>
```

Rules:

- Use the subject that the fact describes, not only the owning scope.
- For time-sensitive facts, keep the key stable and update metadata/text when the fact changes.
- Do not use a date-only key unless the fact itself is an event about that date.

## source_refs Requirement

`source_refs` must include an authoritative source: user-confirmed statement, project instruction, approved doc, durable system record, or source with a stable locator. If the fact is from a tool output, source refs must describe what the tool output proves and why it is durable enough.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "FACT"
title: "New customer data regions require data residency review"
semantic_key: "fact:customer-data:data-residency-review:new-storage-region"
source_refs:
  - type: "policy_document"
    id: "data-residency-policy"
    locator: "region expansion requirement"
    observed_at: "2026-06-03"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Customer data storage region review"
  purpose: "plan-storage-region-changes"
  typeSpecific:
    factSubject: "Customer data storage region approval"
    sourceAuthority: "Approved data residency policy"
    stability: "time_sensitive"
    timeBoundary: "Effective under the policy observed on 2026-06-03; review when the policy or supported regions change."
    reviewAt: "2026-09-03"
text: |
  Fact: New customer data storage regions require data residency review before use.

  Source authority:
  - The approved data residency policy defines the review requirement.

  Stability and time boundary:
  - Time-sensitive fact observed on 2026-06-03; review when the policy or supported regions change.

  Use by:
  - When planning customer data storage in a new region, check residency review before implementation.

  Do not use when:
  - A later policy or approved region list supersedes the review requirement.

  Evidence:
  - The referenced policy document records the region expansion requirement.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "FACT"
title: "Region is approved"
semantic_key: "fact:region"
source_refs:
  - type: "assistant_guess"
    id: "none"
metadata:
  typeSpecific:
    factSubject: "region approval"
    sourceAuthority: "assistant thought so"
    stability: "stable"
    timeBoundary: ""
text: |
  The region is approved.
```

Problems: weak authority, vague subject, missing time boundary, no review date for a mutable policy fact, and an overbroad unqualified claim.
