# Feature Evolution Log: Right Context Sidebar

## Summary

- feature_slug: right-context-sidebar
- status: evolved

## Change History

- 2026-05-26: Browser-comment evolution changed desktop right-sidebar entry behavior to default collapsed and moved desktop collapse/reopen handles to centered boundary positions.

## Downstream Artifact Migration

- AC-RCS-01 proof rows and evidence now describe default-collapsed route entry and non-persisted expanded state instead of persisted collapsed preference.
- AC-RCS-05 proof rows and evidence were narrowed back to shell-owned placement and context preservation.

## Residual Risks

- Existing route-local state can still preserve a user-opened sidebar during a single page stay; validation must continue to verify route entry defaults collapsed.
