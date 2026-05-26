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
- 2026-05-15: reopened AC-AUTH-05 and AC-AUTH-08 again to remove the transitional cookie fallback and converge on the standard browser contract: protected APIs now accept only `Authorization: Bearer <access-jwt>`, login or refresh return the access JWT in the response body, refresh stays in `HttpOnly` cookies, and middleware continues injecting request-scoped authenticated-user context.
- 2026-05-22: reopened AC-AUTH-08 to stop idle access-token expiry from discarding in-progress frontend work; the shared auth client now schedules silent renewal before expiry, retries the first expired-request 401 after refresh, and skips redundant refresh calls when a newer bearer token is already in memory.
- 2026-05-26: added AC-AUTH-09 so authenticated users with no assigned spaces are blocked on an authorization-request page before protected shell navigation or business pages render.
- 2026-05-26: added AC-AUTH-10 so empty deployments initialize a usable `admin` platform administrator with fixed `ADMIN` role, root workspace access, password-reset requirement, and frontend no-space bypass for platform admins.

## Downstream Artifact Migration

- AC-AUTH-02 and AC-AUTH-04 browser evidence must be refreshed because the live provider code and login CTA changed from `dev` or `OAuth2` to `github` or `GitHub`.
- Backend local runtime guidance now depends on `tron-backend/.env.example`, `tron-backend/.env`, and `make be-run` sourcing behavior.
- AC-AUTH-02 backend evidence must now prove first-login auto-provision and second-login provider-subject reuse instead of pre-existing email-only resolution.
- AC-AUTH-05 and AC-AUTH-08 backend and browser evidence now must prove strict bearer-only protected-route auth, refresh-cookie-only bootstrap after reload, and downstream request-context extraction without any access-cookie fallback.
- AC-AUTH-08 browser evidence now must additionally prove idle access renewal and first-401 recovery on protected admin routes without a redirect to `/login`.
- AC-AUTH-09 browser evidence must prove `spaceCodes=[]` users see the authorization-request page on protected routes, cannot see shell navigation or workspace switching, and can still log out.
- AC-AUTH-10 backend evidence must prove empty-database startup inserts only missing bootstrap admin, role, root workspace, assignment, and owner-grant rows, and backfills related grants for an existing `admin` user without duplicating it; browser evidence must prove `ADMIN` bypasses the no-space page.

## Residual Risks

- Residual risk is limited to future non-user-management domains adopting the same auth guard and request-context conventions, future non-GitHub providers needing their own verified profile mapping rules, and genuine refresh-session expiry still requiring re-authentication once the backend can no longer rotate the session.
