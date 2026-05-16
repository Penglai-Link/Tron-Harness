# Test Plan: Auth

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-15

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-AUTH-01 | layer: integration | case: internal login accepts an active username or email and seeds authenticated cookies | red: login endpoint is missing or valid internal credentials fail | green: login returns a session payload and reusable cookies for an active operator | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-02 | layer: integration | case: GitHub OAuth2 callback reuses a linked subject, matches by verified email, or auto-provisions a first-login internal user and creates the same session model | red: GitHub authorize or callback flow cannot create or resolve the internal user needed for session issuance | green: GitHub callback reuses or auto-provisions the internal user and issues authenticated cookies | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-03 | layer: browser | case: anonymous navigation to a protected route redirects to login and preserves redirect intent | red: anonymous navigation renders protected content directly | green: protected navigation redirects to login and resumes the intended route after sign-in | interaction: navigate anonymous browser to `/dashboard` -> redirect to `/login` -> sign in -> dashboard visible | command_ref: CMD-FE-AUTH-BROWSER.
- AC-AUTH-04 | layer: frontend | case: login page exposes internal credentials, a GitHub OAuth2 entry, loading states, and visible auth failures | red: current login page only simulates navigation and lacks real auth feedback | green: login page submits real auth requests and shows busy-state plus error states | interaction: input credentials or click the GitHub provider -> visible busy-state feedback or visible auth failure feedback | command_ref: CMD-FE-AUTH-BROWSER.
- AC-AUTH-05 | layer: api | case: login and refresh issue access JWTs in response bodies, protected APIs accept only bearer access tokens, and logout or refresh preserve the refresh-cookie contract | red: auth endpoints still depend on access-token cookies, ignore `Authorization: Bearer`, or fail to expose current-user context | green: auth grant responses return access tokens, protected APIs reject cookie-only access attempts, downstream handlers can read the authenticated operator, and refresh or logout keep stable cookie behavior | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-06 | layer: database | case: refresh sessions and OAuth2 identity links persist without plaintext session material | red: schema lacks durable auth session or identity link storage | green: auth persistence stores hashed or opaque auth material only | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-07 | layer: security | case: archived users, wrong passwords, bad refresh tokens, and invalid OAuth2 state are rejected consistently | red: invalid auth attempts succeed or return unstable auth errors | green: invalid auth attempts return stable errors and no authenticated session | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-08 | layer: browser | case: authenticated operators can reach existing user-management routes with header-only access tokens, browser reload rebuilds auth state through refresh, and anonymous operators are blocked after logout | red: route protection breaks existing shell navigation, bearer JWT bootstrap fails after page reload, or anonymous admin API access is still allowed | green: user-management stays usable after sign-in, a page reload silently refreshes access through the refresh cookie, account identity moves behind the avatar menu, and logout still redirects to login | interaction: sign in -> open `/admin/users` -> reload page -> user-management remains available -> menu shows profile and logout -> revisit route redirects to `/login` after logout | command_ref: CMD-FE-AUTH-BROWSER.

## Command Index

- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend or schema changes | expect: focused auth integration, bearer-context, contract, and persistence tests pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend compilation changes | expect: backend auth modules compile cleanly.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend or route changes | expect: typecheck and build succeed with auth guard changes.
- CMD-FE-AUTH-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test auth-smoke --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: redirect, login, logout, GitHub callback, and user-management route evidence with concrete browser interactions.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/auth --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: auth feature workspace validates end to end.

## Evidence Rules

- Every `AC-*` row in `spec/acceptance.md` must keep one matching proof row.
- Browser proof must record the visible route transition, user inputs, and protected-route outcome in final reports.
- Backend auth proof must confirm both success and rejection paths because bearer access control and refresh rotation are security-sensitive.
- REVIEW reuses `CMD-FE-AUTH-BROWSER` evidence unless the frontend auth surface changes again or the evidence becomes stale.
