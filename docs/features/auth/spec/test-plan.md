# Test Plan: Auth

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-15

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-AUTH-01 | layer: integration | case: internal login accepts an active username or email and seeds authenticated cookies | red: login endpoint is missing or valid internal credentials fail | green: login returns a session payload and reusable cookies for an active operator | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-02 | layer: integration | case: OAuth2 callback links or reuses an active internal user and creates the same session model | red: authorize or callback flow cannot resolve an active internal user | green: callback resolves an active user and issues authenticated cookies | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-03 | layer: browser | case: anonymous navigation to a protected route redirects to login and preserves redirect intent | red: anonymous navigation renders protected content directly | green: protected navigation redirects to login and resumes the intended route after sign-in | interaction: navigate anonymous browser to `/dashboard` -> redirect to `/login` -> sign in -> dashboard visible | command_ref: CMD-FE-AUTH-BROWSER.
- AC-AUTH-04 | layer: frontend | case: login page exposes internal credentials, OAuth2 entry, loading states, and visible auth failures | red: current login page only simulates navigation and lacks real auth feedback | green: login page submits real auth requests and shows busy-state plus error states | interaction: input credentials or click OAuth2 provider -> visible busy-state feedback or visible auth failure feedback | command_ref: CMD-FE-AUTH-BROWSER.
- AC-AUTH-05 | layer: api | case: providers, session, refresh, and logout follow the uniform envelope and cookie rotation contract | red: auth endpoints are missing or do not rotate or clear session cookies | green: providers, session, refresh, and logout return stable envelopes and cookie behavior | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-06 | layer: database | case: refresh sessions and OAuth2 identity links persist without plaintext session material | red: schema lacks durable auth session or identity link storage | green: auth persistence stores hashed or opaque auth material only | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-07 | layer: security | case: archived users, wrong passwords, bad refresh tokens, and invalid OAuth2 state are rejected consistently | red: invalid auth attempts succeed or return unstable auth errors | green: invalid auth attempts return stable errors and no authenticated session | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTH-08 | layer: browser | case: authenticated operators can reach existing user-management routes and anonymous operators are blocked after logout | red: route protection breaks existing shell navigation or anonymous admin API access | green: user-management stays usable after sign-in and redirects to login after logout | interaction: sign in -> open `/admin/users` -> list request succeeds -> logout -> revisit route redirects to `/login` | command_ref: CMD-FE-AUTH-BROWSER.

## Command Index

- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify/final | rerun: affected backend or schema changes | expect: focused auth integration, contract, and persistence tests pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend compilation changes | expect: backend auth modules compile cleanly.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend or route changes | expect: typecheck and build succeed with auth guard changes.
- CMD-FE-AUTH-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test auth-smoke --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: redirect, login, logout, callback, and user-management route evidence with concrete browser interactions.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/auth --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: auth feature workspace validates end to end.

## Evidence Rules

- Every `AC-*` row in `spec/acceptance.md` must keep one matching proof row.
- Browser proof must record the visible route transition, user inputs, and protected-route outcome in final reports.
- Backend auth proof must confirm both success and rejection paths because cookies and refresh rotation are security-sensitive.
- REVIEW reuses `CMD-FE-AUTH-BROWSER` evidence unless the frontend auth surface changes again or the evidence becomes stale.
