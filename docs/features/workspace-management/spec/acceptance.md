# Acceptance: Workspace Management

## Acceptance Index

- AC-WS-01 [must] [business] Operators can create root and child workspaces, inspect workspace detail, and browse the hierarchy through `/admin/workspace`; proof: @ref:reports/frontend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-02 [must] [business] Workspace move and create operations reject self-parenting, cycles, archived-parent writes, and any hierarchy deeper than three levels; proof: @ref:reports/backend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-03 [must] [interaction] Operators can replace direct-user workspace access grants, including owner flags, with visible success and failure feedback; proof: @ref:reports/frontend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-04 [must] [interaction] Operators can replace role-based workspace access grants with visible success and failure feedback; proof: @ref:reports/frontend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-05 [must] [business] Effective workspace users are derived as the deduplicated union of direct-user grants and role-derived grants and expose the correct access source label; proof: @ref:reports/backend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-06 [must] [api] Workspace APIs persist hierarchy, direct-user grants, role grants, archive or restore state, and effective-user reads behind the uniform admin envelope; proof: @ref:reports/backend-result.md#evidence-rows, @ref:reports/database-result.md#evidence-rows, and @ref:reports/test-report.md#evidence-rows.
- AC-WS-07 [must] [interaction] `/admin/workspace` keeps the authenticated shell mounted, updates visible state without document reload, and preserves clear mode changes between overview, access, and create flows; proof: @ref:reports/frontend-result.md#evidence-rows and @ref:reports/test-report.md#evidence-rows.
- AC-WS-08 [must] [business] Archiving a workspace archives its subtree and restoring it preserves the prior grant configuration while enforcing active-ancestor restore rules; proof: @ref:reports/backend-result.md#evidence-rows, @ref:reports/database-result.md#evidence-rows, and @ref:reports/test-report.md#evidence-rows.

## Open Decisions

- None.
