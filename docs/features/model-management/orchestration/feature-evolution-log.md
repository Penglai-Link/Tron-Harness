# Feature Evolution Log: Model Management

## Summary

- feature_slug: model-management
- The active Lite Spec base branch already carried an in-progress model-management implementation before this request.
- This evolution rewrites the workspace model contract around current-workspace `Local / Inherited / Override` semantics, slot-based defaults, and latest-only persistence with no restore/remove compatibility path.

## Change History

| date | change |
| --- | --- |
| 2026-05-22 | Created the dedicated `docs/features/model-management` workspace for the earlier implicit-platform-inheritance slice. |
| 2026-05-23 | Re-entered feature evolution at the contract layer and rewrote the feature docs to the latest-only `Local / Inherited / Override` model, local status patches, and slot-based defaults. |

## Downstream Artifact Migration

- Workspace catalog logic must stop mapping stored platform rows directly to the workspace `Source` label.
- Frontend pages and mocks must stop exposing inherited-row remove semantics.
- Default resolution must consider ancestor workspace defaults before platform fallback.
- Legacy workspace platform-shadow and restore persistence must be cleaned out rather than translated.

## Residual Risks

- The current agent UI remains mock-driven, so downstream runtime consumers still need a separate delivery slice to call the workspace default contract.
