# Acceptance: App Management

## Acceptance Index

- AC-APP-01 [must] [interaction] `/apps` shows only the currently selected workspace registry by default, supports search/runtime-role/status filters, paged results, and does not expose import or sync actions; proof: pending.
- AC-APP-02 [must] [interaction] `/apps/new` creates an app in the currently selected workspace and routes to `/apps/detail`; proof: pending.
- AC-APP-03 [must] [interaction] `/apps/detail` and `/apps/edit` let operators inspect and update mutable app metadata without changing workspace ownership or app identity; proof: pending.
- AC-APP-04 [must] [business] Apps support `Active | Archived` lifecycle, archived apps disappear from task assignment options, and restore makes them selectable again; proof: pending.
- AC-APP-05 [must] [api] Backend app APIs enforce workspace scoping, duplicate-name protection, query filtering, and lifecycle endpoints through the uniform response contract; proof: pending.
- AC-APP-06 [must] [regression] Task create/detail use active workspace app options, reject invalid or foreign app codes on write, and preserve legacy codes until the user explicitly changes app selection; proof: pending.

## Open Decisions

- None.
