# Feature Spec: {{FEATURE_NAME}}

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: pending
- best_practice_defaults_applied: pending
- user_confirmation_status: waiting_human_decision

Summarize the lead agent's first-pass interpretation, recommended defaults, assumptions, and blocking confirmation questions. This section must exist before task planning.

## Feature Identity

- feature_slug: {{FEATURE_SLUG}}
- feature_name: {{FEATURE_NAME}}
- status: draft
- owner: {{OWNER}}
- source_request: {{SOURCE_REQUEST}}
- main_agent: first agent that activated this skill

## Requirement Description

Describe the user-visible requirement, business goal, target users, and expected outcome.

## Goals And Non-Goals

List measurable goals, non-goals, assumptions, and constraints.

## User Roles And Permissions

Describe roles, permissions, tenancy, visibility rules, and data access boundaries.

## Page Descriptions

Describe each page, route, entry point, navigation path, primary actions, empty states, loading states, and error states.

## Domain Model

Model the domain by identifying aggregates, aggregate roots, entities, value objects, domain services, domain events, invariants, state transitions, and business rules within each bounded context.

## Module And Boundary Map

List affected frontend modules, backend modules, shared libraries, generated clients, adapters, external services, ownership boundaries, and candidate common capabilities that may belong in a shared module.

## Repository Findings

Record existing docs, modules, routes, migrations, tests, package manifests, CI commands, and related feature folders discovered during inspection.

## Feature Dependencies

Before completing this section, scan sibling feature specs under the feature root, normally `docs/features/*/spec/feature-spec.md`, excluding the current feature. Use the content of those `feature-spec.md` files to decide whether this feature has a strong dependency on another feature.

A strong feature dependency means the current feature cannot be designed, implemented, tested, or safely evolved without another feature's domain model, API/schema contract, workflow state, permission model, migration, generated client, shared component, or acceptance behavior.

Only record strong dependencies. The dependency feature name must come from the dependency feature's `Feature Identity` or title. Do not infer it from a folder name alone.

| dependency feature name | feature slug | evidence from feature-spec.md | required contract or artifact | impact on current feature | decision/status |
| --- | --- | --- | --- | --- | --- |
| {{dependency feature name}} | {{feature-slug}} | {{section or fact from dependency feature-spec.md}} | {{API/schema/state/permission/component/AC}} | {{why current feature depends on it}} | waiting_human_decision |

If no strong dependency exists, record `Strong feature dependencies: none found after scanning <n> feature-spec.md files under <features-root>.` with a short rationale.

## Open Decisions

Before writing `None`, identify whether this feature introduces reusable capability that may be needed by other features, bounded contexts, frontend pages, backend services, generated clients, adapters, or test utilities.

Ask the user whether to extract a candidate into a common module when it has cross-feature reuse potential, duplicates existing behavior, centralizes policy or validation, or would otherwise create repeated implementation across modules.

| decision id | candidate common capability | reuse signal | affected modules | proposed shared module | recommendation | user decision |
| --- | --- | --- | --- | --- | --- | --- |
| OD-COMMON-01 | {{capability}} | {{cross-feature use, existing duplication, shared policy, shared validation, shared adapter, shared test utility}} | {{modules}} | {{module/package/path}} | extract now / defer / keep feature-local | waiting_human_decision |

If no candidate common capability exists, record `Common capability extraction: no candidate found` with a short rationale based on repository findings.
