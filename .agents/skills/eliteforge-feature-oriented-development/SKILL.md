---
name: eliteforge-feature-oriented-development
description: Orchestrate EliteForge feature-oriented development for new feature delivery or existing feature evolution. Use only when the user explicitly asks for this skill, or when the current project explicitly follows 璀璨工坊规范 / eliteforge specification and the user wants requirements-to-development-to-testing workflow, TDD-first implementation, database/schema design, API contracts, acceptance criteria, multi-agent execution, git worktree isolation, or review evidence.
---

# EliteForge Feature-Oriented Development

## Activation Gate

Use this skill only when one condition is true:

- the user explicitly asks to use `eliteforge-feature-oriented-development`
- the current project or user request explicitly states `璀璨工坊规范` or `eliteforge specification`, and the user asks for feature-oriented requirements-to-delivery workflow, TDD-first implementation, multi-agent delivery, database/API design, isolated worktrees, or evolution of an existing feature

If neither condition is true, do not activate this skill; handle the request with general capabilities instead.

## Lead Agent Identity

The agent that first activates this skill is the **lead agent**. The lead agent owns planning, decision gates, feature workspace files, task decomposition, worker assignment, worktree isolation, integration, validation, review synthesis, and user communication. For behavior-changing work, RED tests and GREEN implementation belong to worker subagents in their own git worktrees unless the user explicitly disables multi-agent execution.

## Goal

Maintain an auditable `docs/features/<feature-slug>/` workspace for either 0-1 delivery or later feature evolution.

Required chain: request -> selected workflow -> confirmed specs/test boundary/test plan -> tasks and worktrees -> RED/GREEN/refactor/review evidence. Completion requires every current acceptance criterion to map to design decisions, fine-grained TDD cycles, implementation evidence, test evidence, integration evidence, and review status. Evolution must preserve old decisions, tasks, and evidence as retained, superseded, replaced, or not applicable.

For both new feature creation and feature evolution, `spec/feature-spec.md` must include strong feature dependencies found by scanning existing `feature-spec.md` files under the feature root, normally `docs/features/*/spec/feature-spec.md`. If a strong dependency exists, carry the dependency feature name into the current feature spec, downstream impact notes, task context, and worker prompts.

## Workflow Selection

Use **Workflow A: 0-1 Feature Delivery** when there is no existing feature workspace, or the user is starting a new feature from requirement to delivery.

Use **Workflow B: Feature Evolution** when the user changes, refactors, extends, narrows, or reopens an existing `docs/features/<feature-slug>/` workspace. If multiple existing feature folders could match, inspect them and ask the user to choose.

Both workflows share the same workspace, templates, validators, TDD rules, and worktree constraints. Evolution re-enters the 0-1 gates from the earliest affected design point instead of editing downstream files opportunistically.

## Workspace Contract

Use `docs/features/<feature-slug>/` unless the user provides an existing feature directory. The canonical workspace contains `spec/`, `tasks/`, `orchestration/`, and `reports/`; use `assets/` templates for new files and validators for phase completeness.

Derive `<feature-slug>` from the real product feature name in lowercase kebab-case. Worker worktrees and branches are agent-first: `../.eliteforge-worktrees/<repo-name>/eliteforge/<agent-id>/<feature-slug>/`, `eliteforge/<agent-id>/<feature-slug>`, and `eliteforge/integration/<feature-slug>`.

If files already exist, preserve their decisions and update in place instead of overwriting history.

Before creating or evolving `spec/feature-spec.md`, determine the feature root from the current feature directory. Scan sibling `*/spec/feature-spec.md` files, excluding the current feature, and identify strong dependencies from their content. A strong dependency exists when the current feature cannot be designed, implemented, tested, or safely evolved without another feature's domain model, API/schema contract, workflow state, permission model, migration, generated client, shared component, or acceptance behavior. Record only strong dependencies and include the dependency feature name from that feature spec.

## Resource Paths

Resolve `assets/`, `references/`, and `scripts/` relative to this `SKILL.md`, not the target repository. Before running a bundled helper, set `skill_dir` to the active skill directory and call `"$skill_dir/scripts/<script-name>"`.

## Non-Negotiable Rules

1. **Draft before interrogation.** During initial documentation, the lead agent must first produce an opinionated draft from the user's requirement, repository findings, and best practices. Do not start with a long questionnaire. Ask only focused confirmation questions after the draft exists.
2. **Confirm progressively.** Mark assumptions, recommendations, and open decisions in docs. Ask at most 3 to 5 blocking questions per confirmation round.
3. **Scan feature dependencies.** For creation and evolution, inspect existing sibling `feature-spec.md` files under the feature root. If strong dependencies exist, record the dependency feature names in `spec/feature-spec.md` and carry them into downstream impact notes and worker context.
4. **Keep workflow gates.** Follow the selected workflow reference for phase boundaries. Do not move downstream artifacts into earlier gates.
5. **Separate ACs from TDD cycles.** Acceptance criteria are stable验收口径; TDD cycles are smaller executable slices covering core business logic, boundaries, errors, permissions, integration, and regression risks.
6. **Keep lead and worker roles separate.** The lead plans, delegates, integrates, validates, and reviews. Workers write tests or implementation in dedicated git worktrees.
7. **Enforce worktree isolation.** No worker edits the lead/root worktree, no two workers share a worktree or branch, and worker names must use `eliteforge/<agent-id>/<feature-slug>`.
8. **RED before GREEN.** GREEN implementation may start only after matching RED failure evidence exists and has been merged or replayed into the integration branch.
9. **Do not hide execution gaps.** If subagents/worktrees are unavailable, shared-file races appear, or evolution invalidates prior artifacts, record the blocker/status and stop for user decision.

## Reference Index

Load references only when the selected workflow or phase needs them:

| need | reference |
| --- | --- |
| New feature delivery workflow | `references/zero-one-delivery.md` |
| Existing feature evolution workflow | `references/feature-evolution.md` |
| Initial progressive drafting | `references/initial-design.md` |
| Database design guidance | `references/database-design.md` |
| Test scope gate | `references/test-scope-gate.md` |
| RED/GREEN workflow rules | `references/tdd-workflow.md` |
| Subagent role prompts and batching | `references/subagent-orchestration.md` |
| Worktree setup and integration | `references/multi-agent-worktree-orchestration.md`, `references/git-worktree-orchestration.md` |

## Workflow A: 0-1 Feature Delivery

Read `references/zero-one-delivery.md`: progressive design gates -> approved executable test plan -> task breakdown -> worktree provisioning -> RED -> GREEN -> refactor/verify -> final review.

## Workflow B: Feature Evolution

Read `references/feature-evolution.md` before changing an existing feature workspace.

Summary: orient and freeze active dispatch -> classify change -> reopen earliest affected Workflow A gate -> update downstream specs/test plan/tasks/reports -> preserve prior evidence status -> resume from the affected execution phase. Run the matching `--spec-stage` validator after each reopened gate, and `--complete-spec` whenever acceptance, test scope, or test plan changes.

## Completion Gate

The workflow is complete only when active validation passes, reached gates have approved or explicitly pending decisions, ACs map to TDD cycles and executable evidence, behavior-changing work has RED/GREEN/refactor/review/worktree evidence, and unresolved blockers are recorded.

Feature Evolution additionally requires change classification, downstream impact analysis, affected-gate validation, stale artifact status, and links between old/new evidence and changed ACs.
