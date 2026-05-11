# Initial Design Drafting

Use this reference during Phase 1 before asking the user to complete each progressive feature document gate.

## Principle

Initial design is agent-drafted, not questionnaire-first. Infer each gated draft from:

- the user's stated requirement
- repository structure and existing conventions across API, database, frontend, tests, and CI
- sibling `feature-spec.md` files under the feature root, used to identify strong feature dependencies by feature name
- best practices for the detected stack, risk level, permissions, data sensitivity, and operational impact

The draft may be imperfect, but it must be concrete enough for the user to approve, reject, or correct.

## Progressive Draft Gates

Create or update feature documents in this order:

1. `docs/features/<feature-slug>/spec/feature-spec.md`
2. `docs/features/<feature-slug>/spec/database-design.md`
3. `docs/features/<feature-slug>/spec/api-contract.md`
4. `docs/features/<feature-slug>/spec/acceptance.md` and `docs/features/<feature-slug>/spec/test-scope.md`

Stop for confirmation after each gate unless the user explicitly authorized best-practice defaults. Do not create later spec files, final task files, worktree plans, execution reports, or `spec/test-plan.md` early.

## Draft Content Requirements

Each draft should distinguish inferred facts, best-practice recommendations, missing-information assumptions, common capability extraction candidates, high-impact confirmation decisions, and lower-risk defaults.

Keep unresolved non-blocking details in `Open Decisions` or `Assumptions` instead of interrupting progress.

## Confirmation Style

After each gated draft, ask at most 3 to 5 focused questions. Prefer this pattern:

```text
I drafted <current-file> with these assumptions:
1. ...
2. ...
3. ...

Please confirm or correct these blocking decisions:
1. ...
2. ...
3. ...
```

## Best-Practice Defaults

When the user asks to proceed with best practices, use defaults such as stable acceptance IDs, explicit non-goals, API/error contracts, migration/rollback notes, unit tests for rules, integration/API contract tests, migration tests, security/permission tests, and one runtime/browser path for user-facing UI.

Record these defaults in the relevant spec files. Use `status: approved_with_defaults` only when the user explicitly allows recommended defaults.
