# Feature Evolution Log: Auth

## Summary

- feature_slug: auth
- workflow: feature evolution after initial delivery
- status: completed

## Change History

- 2026-05-15: completed the first delivery pass with backend auth APIs, SQLite auth persistence, frontend route guards, login and callback pages, user-management regression coverage, and final verification evidence.
- 2026-05-15: reopened AC-AUTH-02 and AC-AUTH-04 to replace the placeholder dev OAuth2 runtime path with GitHub OAuth App support, GitHub-labeled frontend smoke evidence, and backend environment templates.
- 2026-05-15: normalized the interrupted recovery by recreating `feature/clouds3n/auth` in place from the dirty root worktree before finishing the GitHub OAuth pass.
- 2026-05-15: updated `make be-run` to source `tron-backend/.env` when present so local GitHub OAuth configuration matches `tron-backend/.env.example` and the Vite `/api` proxy callback path.
- 2026-05-15: reopened AC-AUTH-02 after runtime validation showed link-only GitHub OAuth forced manual database patching; refactored callback semantics to auto-provision a minimal internal user on first GitHub login and reuse durable provider-subject links thereafter.

## Downstream Artifact Migration

- AC-AUTH-02 and AC-AUTH-04 browser evidence must be refreshed because the live provider code and login CTA changed from `dev` or `OAuth2` to `github` or `GitHub`.
- Backend local runtime guidance now depends on `tron-backend/.env.example`, `tron-backend/.env`, and `make be-run` sourcing behavior.
- AC-AUTH-02 backend evidence must now prove first-login auto-provision and second-login provider-subject reuse instead of pre-existing email-only resolution.

## Residual Risks

- Residual risk is limited to future non-user-management domains adopting the same auth guard and cookie-backed request conventions, plus future non-GitHub providers needing their own verified profile mapping rules.
