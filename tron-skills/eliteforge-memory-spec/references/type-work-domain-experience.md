# Type: WORK_DOMAIN_EXPERIENCE

Use `WORK_DOMAIN_EXPERIENCE` for durable lessons about how to avoid, diagnose, or recover from recurring execution failures within an identifiable professional domain.

Do not use it for user preferences, workspace rules, project facts, raw error logs, arbitrary agent experience, or a one-off mistake without a reusable corrective action. If no reusable professional domain can be named, do not use this type.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Lesson: When <trigger>, avoid <failureMode> by <correctiveAction>.

Root cause:
- <why the failure happened>

Corrective action:
- <what the future agent should do instead>

Validation:
- <how the corrective action was proven>

Recurrence prevention:
- <guardrail, check, or habit to prevent repeat failure>

Do not apply when:
- <situations where the lesson does not generalize>

Evidence:
- <brief evidence summary that points to source_refs>
```

The first line must be actionable. Do not store a complaint or error message without the reusable fix.

## metadata.typeSpecific

Required fields:

- `trigger`: condition that makes the lesson relevant.
- `failureMode`: the recurring or costly failure to avoid.
- `rootCause`: why it happened.
- `correctiveAction`: what to do differently.
- `validationResult`: how the fix was checked.
- `recurrencePrevention`: durable guardrail for future work.

The professional domain must be named outside `metadata.typeSpecific`, such as in `metadata.subject`, `metadata.purpose`, or `semantic_key`, unless the current schema explicitly defines a domain/category field.

## semantic_key Pattern

Use:

```text
work-domain-experience:<domain-slug>:<trigger-slug>:<failure-mode-slug>
```

Rules:

- Use the narrow reusable professional domain as the first semantic segment.
- Do not invent a `scope_code`; use the domain in `metadata.subject`, `metadata.purpose`, or `semantic_key`.
- Key by trigger and failure mode, not by the exact error string.
- Update the same key when adding stronger validation or a better guardrail.

## source_refs Requirement

`source_refs` must include the failure evidence and the validation evidence when both exist. A single observation without root cause, validation, or a professional domain named in the record subject, purpose, or semantic key should remain a note, not an approved work-domain experience.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "WORK_DOMAIN_EXPERIENCE"
title: "Pin validation context before diagnosing environment-dependent failures"
semantic_key: "work-domain-experience:software-validation:env-inheritance:misdiagnosed-product-failure"
source_refs:
  - type: "investigation_note"
    id: "validation-env-inheritance-note"
    locator: "root cause and validation"
    observed_at: "2026-06-02"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Software validation environment inheritance"
  purpose: "prevent-misdiagnosed-validation-failures"
  typeSpecific:
    trigger: "Running validators whose behavior depends on external environment configuration."
    failureMode: "Inherited host configuration makes a validation failure look like product behavior."
    rootCause: "The validation process inherited host settings instead of declaring the required context."
    correctiveAction: "Pin the required validation context before diagnosing application behavior."
    validationResult: "The same check became stable after the required context was declared explicitly."
    recurrencePrevention: "Inspect validation context before changing product code for environment-dependent failures."
text: |
  Lesson: When running environment-dependent validators, avoid misdiagnosing host configuration as product behavior by pinning the validation context first.

  Root cause:
  - The validation process can inherit host settings and silently change expected behavior.

  Corrective action:
  - Declare the required validation context before interpreting failures as product defects.

  Validation:
  - The same check became stable after the required context was declared explicitly.

  Recurrence prevention:
  - Inspect validation context before changing product code for environment-dependent failures.

  Do not apply when:
  - The validation intentionally inherits host configuration and documents that intent.

  Evidence:
  - The referenced investigation note records the root cause and validated corrective action.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "WORK_DOMAIN_EXPERIENCE"
title: "Validation is flaky"
semantic_key: "experience:flaky"
source_refs:
  - type: "terminal"
    id: "failed-test-output"
metadata:
  typeSpecific:
    trigger: "validation"
    failureMode: "flaky"
    rootCause: "unknown"
    correctiveAction: "try again"
    validationResult: "not checked"
    recurrencePrevention: ""
text: |
  Validation sometimes fails. Rerun it.
```

Problems: no named professional domain in subject, purpose, or semantic key; no real root cause; no validated corrective action; no prevention; overgeneralized trigger; and no durable lesson.
