# Test Plan: Model Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-23

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-MODEL-01 | layer: api | case: current-workspace source classification and source lineage display work across ancestor lineage | red: workspace catalog still maps platform rows directly to `Override`, a parent override cannot flow to a child as inherited state, or `Inherited From` only shows a single workspace name instead of the root-to-source path | green: backend tests prove current-workspace rows resolve as `LOCAL`, `INHERITED`, or `OVERRIDE`, platform-sourced rows keep an empty `sourceWorkspacePath`, and workspace-sourced rows expose `sourceWorkspacePath` ordered root-to-source | interaction: create parent local root -> load child workspace catalog -> verify child shows `Inherited` and `Inherited From = Enterprise / Platform`; create parent override -> verify descendants inherit the override source path; current-workspace local rows still show `Local` | command_ref: CMD-MODEL-BE-TEST.
- AC-MODEL-02 | layer: browser | case: workspace actions and filters follow latest-only local, inherited, and override semantics | red: inherited rows still expose `Remove` or `Delete`, disabling an inherited row flips the source label to `Override`, override rows have no revert path, detail-page verify still requires save first, or protocol filtering still combines protocol and type in one dropdown | green: browser and API proof show inherited rows can be disabled or edited into overrides, local rows can be deleted, override rows can be reverted, a disabled inherited row still renders `Inherited` with `Disabled` status, workspace or platform detail pages can verify draft edits before save, and workspace filters expose independent `Protocol` and `Type` dropdowns | interaction: open `/models` -> verify `Protocol` and `Type` are separate filters -> filter by protocol only -> filter by type only -> disable an inherited row -> verify `Source=Inherited` and `Status=Disabled` -> edit the same row into an override -> verify `Source=Override` -> revert -> open workspace detail edit mode -> change draft fields -> verify before save -> cancel -> confirm the saved record remains unchanged | command_ref: CMD-MODEL-BROWSER.
- AC-MODEL-03 | layer: api | case: enabled local, inherited, and override rows can all be set as workspace defaults, and default resolution follows workspace lineage before platform fallback | red: inherited or override rows are rejected as workspace default targets, or a child workspace ignores an ancestor workspace default for the same slot | green: backend and browser proof show current, ancestor, and platform defaults resolve by visible slot `modelCode`, and a current-workspace override still satisfies the same slot-based default assignment | interaction: set platform default -> set parent workspace default -> open child workspace catalog -> verify inherited default -> create child override on the same slot -> set default from the override -> verify the child default changes without mutating parent or platform defaults | command_ref: CMD-MODEL-BE-TEST.
- AC-MODEL-04 | layer: backend | case: latest-only persistence drops old restore or shadow semantics and exposes only current relation kinds | red: workspace APIs, mock data, or migrations still rely on restore endpoints, hidden remove rows, or legacy workspace `source_kind = platform` shadow records | green: backend cleanup and frontend mock proof show only latest relation kinds are used, invalid legacy workspace defaults are removed, and no restore endpoint or remove-shadow route remains part of the contract | interaction: not applicable | command_ref: CMD-MODEL-BE-TEST.

## Command Index

- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: workspace and settings model pages compile and build.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: model service, routing, and persistence compile cleanly.
- CMD-MODEL-BE-TEST | command: rtk cargo test --manifest-path tron-backend/Cargo.toml --test model_management_api -- --nocapture | cost: medium | when: red/green/verify/final | rerun: affected backend or persistence changes | expect: workspace lineage, override, status patch, default, and latest-only cleanup scenarios pass.
- CMD-MODEL-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test model-management-smoke.spec.ts --reporter=line | cost: slow/browser | when: verify/final | rerun: affected frontend model page or mock/API wiring changes | expect: `/models` reflects current-workspace source semantics and latest-only actions.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/model-management --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: model-management feature docs validate end to end.

## Evidence Rules

- Every `AC-MODEL-*` row must have final evidence in reports.
- Browser evidence must list clicks, inputs, network assertions, visible feedback, and artifacts.
- Backend evidence for AC-MODEL-01 and AC-MODEL-03 must mention ancestor workspace lineage, not only platform inheritance; AC-MODEL-01 evidence must include `sourceWorkspacePath`.
- Backend or database evidence for AC-MODEL-04 must mention the latest-only cleanup path and the removed old relation semantics.
