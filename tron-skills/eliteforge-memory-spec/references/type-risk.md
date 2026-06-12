# Type: RISK

Use `RISK` for a durable hazard that future agents should actively avoid, monitor, or mitigate because it can cause incorrect output, unsafe execution, privacy leakage, data loss, or governance damage.

Do not use it for generic fear, low-impact uncertainty, raw bug reports, or a risk that has no actionable avoidance path.

## Persisted Text Internal Format

Persist `text` as plain Markdown in this order:

The block order and meaning are fixed; localize block labels and content to `metadata.language` when the source or user language is not English.

```md
Risk: <riskTrigger> can cause <impact>.

Impact:
- <what goes wrong and who or what is affected>

Avoidance:
- <specific future behavior that reduces the risk>

Severity and owner:
- <severity> risk owned by <owner>

Applies when:
- <situations where the risk should change behavior>

Do not apply when:
- <situations where the risk is irrelevant or already mitigated>

Evidence:
- <brief evidence summary that points to source_refs>
```

Make the risk actionable. If no future behavior changes, do not persist it as a risk memory.

## metadata.typeSpecific

Required fields:

- `riskTrigger`: condition that activates the risk.
- `impact`: concrete harm or failure mode.
- `avoidance`: specific mitigation or behavior.
- `severity`: severity label such as `high`, `medium`, or `low`.
- `owner`: person, team, agent, or scope responsible for managing the risk.

## semantic_key Pattern

Use:

```text
risk:<scope-code>:<trigger-slug>:<impact-domain-slug>:<avoidance-slug>
```

Rules:

- Use the scope where the risk should influence behavior.
- Key by trigger and impact domain, not by a transient error message.
- If mitigation changes, update the same memory unless the risk trigger changes.

## source_refs Requirement

`source_refs` must include evidence of the risk or an authoritative warning: incident, review finding, security note, data-loss report, privacy review, or explicit owner concern. Pure speculation should not become approved risk memory.

Examples are persisted-content examples, not MCP invocation schemas. They intentionally omit shared envelope fields governed by `SKILL.md`, including `scope`, `scope_code`, `memory_status`, `review_status`, `injectable`, and `metadata.language`.

## Good Type-Specific Payload Example

```yaml
memory_type: "RISK"
title: "Raw source content can leak sensitive context"
semantic_key: "risk:memory-governance:raw-source-content:privacy-leak:redact-before-saving"
source_refs:
  - type: "privacy_review"
    id: "memory-redaction-policy"
    locator: "raw-source-content boundary"
    observed_at: "2026-06-02"
metadata:
  formatVersion: "memory-spec.v1"
  subject: "Raw source content in persisted memory"
  purpose: "prevent-sensitive-memory-leakage"
  typeSpecific:
    riskTrigger: "Persisting raw transcripts, tool output, or copied source content as memory evidence."
    impact: "Sensitive data can enter retrievable long-term memory and reappear in future context."
    avoidance: "Summarize the durable claim and point to evidence refs; redact secrets and avoid copying raw content."
    severity: "high"
    owner: "memory governance owner"
text: |
  Risk: Persisting raw transcripts, tool output, or copied source content as memory evidence can leak sensitive context.

  Impact:
  - Sensitive data can enter retrievable long-term memory and reappear in future context.

  Avoidance:
  - Summarize the durable claim, point to evidence refs, redact secrets, and avoid copying raw content.

  Severity and owner:
  - High risk owned by the memory governance owner.

  Applies when:
  - Creating memory from transcripts, command output, logs, documents, or copied source material.

  Do not apply when:
  - The source content is already public and the persisted memory still summarizes rather than copies it.

  Evidence:
  - The referenced privacy review records the raw-content persistence boundary.
```

## Bad Type-Specific Payload Example

```yaml
memory_type: "RISK"
title: "Providers may fail"
semantic_key: "risk:provider"
source_refs: []
metadata:
  typeSpecific:
    riskTrigger: "using providers"
    impact: "bad things happen"
    avoidance: "be careful"
    severity: "high"
    owner: "everyone"
text: |
  Providers are risky, so be careful.
```

Problems: no evidence, vague trigger, vague impact, no actionable mitigation, and no meaningful owner.
