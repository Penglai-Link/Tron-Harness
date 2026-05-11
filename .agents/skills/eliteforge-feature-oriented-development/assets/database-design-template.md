# Database Design: {{FEATURE_NAME}}

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: pending
- best_practice_defaults_applied: pending
- user_confirmation_status: waiting_human_decision

Summarize the lead agent's first-pass interpretation, recommended defaults, assumptions, and blocking confirmation questions. This section must exist before task planning.

## Database Decision

- status: draft
- impact: unknown
- approved_by: none
- approved_at: none

Use `status: not_applicable` when the feature has no database or persistence impact. Use `status: approved` only after the user or project owner accepts the design.

## Existing Schema Impact

List affected tables, views, indexes, constraints, migrations, repositories, ORM entities, caches, reports, and downstream consumers.

## Proposed Schema

| object | change type | columns or fields | constraints | indexes | notes |
| --- | --- | --- | --- | --- | --- |

## Relationships And Cardinality

Describe foreign keys, ownership, cascade behavior, uniqueness, nullable relationships, and domain invariants.

## Migration Plan

Describe migration order, backward compatibility, generated migration names, data backfills, default values, lock risks, deployment sequencing, and rollback plan.

## Query And Performance Plan

Describe primary query paths, pagination, sorting, filters, index coverage, expected cardinality, concurrency/locking, and performance risks.

## Data Governance

Describe tenancy, permissions, audit fields, soft delete, retention, privacy, encryption/masking, medical/financial/regulatory considerations, and access logging.

## Test Data And Fixtures

Describe seed data, factories, fixtures, Testcontainers or embedded database use, cleanup rules, and migration verification data.

## Database Validation Commands

| command | purpose | expected evidence |
| --- | --- | --- |

## Open Decisions

- None.
