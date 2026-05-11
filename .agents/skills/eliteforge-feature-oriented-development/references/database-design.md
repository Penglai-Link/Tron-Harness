# Database Design Reference

Use this reference when a feature touches persistence, migrations, data ownership, tenancy, permissions, reporting, search, audit, caching, or seed data.

## Required Decision

Every feature must explicitly mark `spec/database-design.md` with one of these statuses:

```text
status: waiting_human_decision
status: approved
status: not_applicable
```

Implementation tasks cannot begin unless status is `approved` or `not_applicable`.

## Design Checklist

When database changes exist, document:

- affected bounded context or module
- table names and ownership
- new or changed columns with type, nullability, default, and semantics
- primary keys, unique keys, foreign keys, check constraints, and business constraints
- indexes and expected query patterns
- migration file naming and ordering
- forward and backward compatibility
- rollback strategy or irreversible-change warning
- data backfill or seed strategy
- transaction, locking, and consistency expectations
- tenancy, authorization, and data visibility rules
- audit fields, soft delete, retention, and privacy rules
- ORM/entity/repository changes
- fixtures and test data strategy
- migration validation commands

## Compatibility Rule

Prefer expand-and-contract migrations for production systems:

1. add compatible structures
2. deploy code that can read/write old and new shape when needed
3. backfill data
4. enforce constraints after data is valid
5. remove old structures after the compatibility window closes

Destructive migrations require user confirmation.
