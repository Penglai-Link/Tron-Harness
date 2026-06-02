# Feature Spec: Model Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: model-management
- feature_name: Model Management
- status: approved_with_defaults
- owner: clouds3n
- source_request: 使用 `eliteforge-codex-superpower` 重构 models-management feature；workspace 模型按 `Local / Inherited / Override` 表达；disabled 只是 status 变化；`Inherited` 和 `Override` 也可以设 default；不保留旧 restore/remove/shadow 兼容。
- main_agent: Codex lead agent
- dependency_features: `workspace-management`, `shared-ui-feedback`
- dependency_contracts: authenticated shell workspace context, existing platform model settings surface, uniform API response envelope

## Requirement

Workspace model management must resolve one effective catalog per workspace from platform models, ancestor workspace-local models, and ancestor overrides. Every visible workspace row must be classified by the current workspace perspective only:

- `Local`: the current workspace owns the record itself and the row is not overriding an upstream slot.
- `Inherited`: the current workspace has no definition override for the slot; the row currently comes from a parent workspace chain or from the platform. A workspace-local status patch may still flip enabled or disabled without changing this source category.
- `Override`: the current workspace explicitly overrides an upstream slot from a parent workspace chain or the platform.

The workspace UI must stop treating inherited platform rows as removable shadows. Inherited rows can be disabled locally, copied into a local model, edited to create an override, and set as default when enabled. Local rows can still be edited or deleted. Override rows can be edited, toggled, set as default, copied into a local model, or reverted back to inherited state.

Workspace defaults must be slot-based rather than shadow-record-based. When an agent or another downstream consumer does not specify a model explicitly, it should resolve the current workspace default for the required `modelType` from the current workspace explicit default first, then the nearest ancestor workspace explicit default, then the platform default or platform fallback.

Platform and workspace verify actions must always validate the current form payload independently from save. Create pages may call a dedicated draft-verify endpoint before any row exists, and detail pages may send the current edit draft for verification without persisting it. Verify must never mutate the stored model definition implicitly.

Platform defaults and workspace defaults are keyed by the coarse `modelType` slot, not by fine-grained capability labels such as `Text Generation` or `Deep Thinking`. Multiple capability labels may map into the same `modelType` slot. If a platform `modelType` slot has no explicit default yet, the first enabled platform root model created in that slot must materialize the platform default assignment automatically.

## Product Source Slices

- selected_feature: `Feature: Model Management`
- selected_page: `Page: /models`; `Page: /models/detail`; `Page: /models/new`; `Page: /settings/models`
- selected_interactions: `Interaction: Browse Effective Workspace Models`; `Interaction: Edit Inherited Model`; `Interaction: Verify Draft Without Save`; `Interaction: Set Workspace Default`
- selected_rules: `Rule: Local Inherited Override Semantics`; `Rule: Slot Based Defaults`; `Rule: Draft First Verify`
- selected_exceptions: `Exception: Inherited Rows Cannot Be Deleted`; `Exception: Disable Does Not Relabel Source`; `Exception: Revert Returns To Inherited`
- selected_acceptance_source: `Acceptance: Model Management`
- selected_workspace: `docs/features/model-management`
- feature_inventory_summary: model management resolves one effective workspace catalog while preserving platform authoring as the upstream source of truth
- shared_requirement_summary: workspace-management supplies active workspace context and shared-ui-feedback supplies toast feedback for verify and mutation actions
- dependency_map_summary: platform model settings feed workspace-effective model resolution, and workspace defaults feed later agent consumers
- implementation_order_summary: backend resolution and default-slot semantics land first, then workspace and platform pages align their visible source labels and verify flows

## Scope Boundary

This feature evolution rewrites the model-management contract across backend resolution logic, frontend workspace model pages, mock data, migrations, focused API tests, browser smoke coverage, and the feature docs themselves.

## Assumptions

- Platform models remain authored only from `/settings/models`.
- Workspace-local models keep `allowInheritance`; that flag still applies only to independently owned workspace-local rows.
- Override rows and status patches are internal persistence details; the workspace UI only exposes the effective row plus its current source classification.
- The current agent UI is still mock-driven, so this slice defines and proves the default-selection contract but does not introduce a new production agent runtime.

## In Scope

- Rebuild workspace catalog resolution into current-workspace `Local / Inherited / Override` semantics across multi-level workspace lineage.
- Allow ancestor override rows to flow to descendants as inherited effective rows.
- Keep local status changes separate from source classification so `disable` can remain `Inherited` when only `enabled` changes in the current workspace.
- Let enabled `Local`, `Inherited`, and `Override` rows all participate in workspace default selection.
- Support draft-first verify on both platform and workspace create or detail pages without implicit save.
- Materialize an explicit platform default assignment when the first enabled platform root model is created in an empty `modelType` slot.
- Treat default-slot identity as the coarse `modelType`, not as a capability combination.
- Replace inherited-row delete or remove semantics with local status toggles and override reverts.
- Remove old restore-route, remove-shadow, and compatibility behavior from API, frontend, mocks, and persistence cleanup.

## Out Of Scope

- Building a production agent execution runtime that consumes workspace defaults end to end.
- Redesigning platform model settings pages beyond the contract needed for inherited defaults.
- Adding a separate governance UI for historical override audit trails.

## Constraints

- All API responses must keep the uniform envelope.
- `/models` remains the workspace-effective catalog route under the authenticated shell workspace context.
- The implementation follows the latest contract only; old workspace platform shadow or restore semantics are not preserved.
- Feature docs may be rewritten completely when the previous wording no longer matches the accepted contract.

## Feature Model

Bounded context:

- Platform model registry plus workspace-effective model resolution.

Entities:

- `PlatformModel`: a platform-owned root model slot.
- `PlatformDefaultAssignment`: a platform-scoped choice of which platform root model should be the default per `modelType`.
- `WorkspaceLocalModel`: a workspace-owned root model slot with optional descendant inheritance.
- `WorkspaceOverrideModel`: a workspace-owned definition override that targets one upstream slot by visible `modelCode`.
- `WorkspaceStatusPatch`: a workspace-owned status-only patch that targets one upstream slot by visible `modelCode`.
- `WorkspaceDefaultAssignment`: a workspace-scoped choice of which visible slot should be the default per `modelType`.

Policies:

- Every visible workspace row is resolved against one stable slot key, exposed as the effective `modelCode`.
- A child workspace inherits the nearest upstream effective definition. If a parent override exists, descendants inherit that override until they create their own override.
- A status-only patch changes only `enabled` for the current workspace view and does not relabel the source as `Override`.
- Platform and workspace default assignments are keyed by coarse `modelType`; capability labels may affect validation or display but do not create separate default slots.
- Create-page and detail-page verify actions always validate the current draft payload without persisting it.
- If a platform `modelType` slot has no explicit default assignment yet, creating the first enabled platform root model in that slot materializes one automatically.
- A workspace default always points at the visible slot key, so a current-workspace override still satisfies the same default assignment.
- Legacy restore or remove shadow data is not part of the active model and may be deleted during cleanup.

## Page And Module Surface

Workspace model management:

- Route surface: `/models`, `/models/detail`, `/models/new`
- Behavior:
  - show `Source` as `Local`, `Inherited`, or `Override`
  - show `Inherited From` as `Platform` for platform-sourced rows, or the full source workspace lineage path for workspace-sourced rows, for example `Enterprise / Platform / Tron Core`
  - filter the effective catalog by search text, status, protocol, and type as separate controls; `Protocol` filters only the protocol enum, while `Type` filters the displayed type or capability set
  - allow `Set Default` for any enabled visible row
  - allow `Disable/Enable` for any visible row
  - allow `Edit` on local rows and on inherited or override rows, where inherited edits create a current-workspace override
  - allow `Verify` on create and detail pages against the current form values without saving
  - allow `Delete` only for local rows
  - allow `Revert` only for override rows

Platform model management:

- Route surface: `/settings/models`, `/settings/models/detail`, `/settings/models/new`
- Behavior:
  - remains the only surface that creates, edits, disables, duplicates, deletes, or chooses defaults for platform root models
  - allow `Verify` on create and detail pages against the current form values without saving
  - keep create-page verify on the current page and do not create a row implicitly
  - choose defaults per coarse `modelType` slot rather than per capability label combination
  - materialize the platform default automatically when the first enabled platform root model is created in an empty `modelType` slot

Backend model API:

- Modules: `tron-backend/src/application/model.rs`, `tron-backend/src/adapters/inbound/http/models/*`, `tron-backend/src/adapters/outbound/persistence/sqlite/*`
- Behavior: expose platform plus workspace draft-verify endpoints, latest-only workspace-effective semantics, coarse-`modelType` slot-based defaults, automatic first-platform-default materialization, and no restore/remove compatibility layer.

Frontend model modules:

- Modules: `tron-frontend/src/domain/model/api/modelApi.ts`, `tron-frontend/src/domain/model/mock/data.ts`, `tron-frontend/src/domain/model/pages/*`, `tron-frontend/tests/model-management-smoke.spec.ts`
- Behavior: render current-workspace source semantics, inherited-from details, draft-first verify flows, and latest-only actions.

## Repository Findings

- The current backend already resolves platform and ancestor-local rows inside one workspace catalog, but it still mixes storage `source_kind` with workspace UI source semantics and only supports platform-scoped workspace overrides.
- The current frontend treats `PLATFORM` as `Override`, which is semantically wrong for inherited platform rows.
- The current detail and list actions still assume inherited platform rows are special removable references rather than effective inherited slots.
- The current agent page remains mock-backed, so default-model behavior must be specified at the model-management contract layer first.

## Open Decisions

- None.
