# Test Plan: Variable Sensitive Data

## Test Plan Decision

- status: approved_with_defaults
- approved_by: baiyihuan
- approved_at: 2026-06-02

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-VSD-01 | layer: backend | case: migration adds sensitive metadata and legacy compatibility | red: migrated variable tables lack sensitivity columns or old rows fail to list | green: backend tests prove both tables add metadata, old rows default non-sensitive, and existing list/mutation behavior still works | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-02 | layer: backend | case: SM4-GCM storage encryption and backend-only decryption | red: sensitive create stores submitted plaintext or decrypt works without required metadata | green: backend tests prove ciphertext storage, required nonce/tag/key metadata, AAD mismatch failure, and backend-controlled decrypt success | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-03 | layer: backend | case: display APIs and search are mask-safe | red: list/detail/search returns or matches hidden plaintext for sensitive rows | green: backend tests prove display VOs return `********`, crypto metadata is absent, and sensitive plaintext is not searchable | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-04 | layer: backend | case: sensitive edit, demotion, and inherited override rules | red: edit preloads plaintext, preserve loses ciphertext, demotion reuses old secret, or inherited override downgrades sensitivity | green: backend tests prove preserve/replace/demotion validation and inherited sensitive override enforcement | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-05 | layer: backend | case: password step-up token and scoped reveal | red: reveal succeeds on session alone, wrong token, expired token, unsupported auth, or wrong scope | green: backend tests prove password verification issues scoped token and reveal returns only one authorized plaintext value | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-06 | layer: backend | case: runtime resolver and service-mode trust policy | red: agent invocation uses masked display values or posts sensitive plaintext to untrusted service runtime | green: backend tests prove runtime resolver returns real values only inside trusted boundary and untrusted service mode fails before POST | command_ref: CMD-BE-VARIABLE-TEST.
- AC-VSD-07 | layer: browser | case: workspace variable UI mask, reveal, hide, and search behavior | red: `/variables` shows or searches sensitive plaintext, lacks reveal verification, or keeps plaintext after navigation | green: browser test proves mask display, password step-up, reveal, hide, search miss, and remask on workspace switch or reload | interaction: open `/variables` -> search by secret substring -> reveal one row with password -> hide -> switch workspace or reload -> verify mask returns | command_ref: CMD-VSD-BROWSER.
- AC-VSD-08 | layer: browser | case: platform variable UI mask, reveal, hide, and edit behavior | red: `/settings/variables` shows plaintext, edit form preloads secret, or save cannot preserve/replace safely | green: browser test proves mask display, reveal dialog, safe edit preserve, replace submission, and remask after navigation | interaction: open `/settings/variables` -> create sensitive value -> edit description preserving value -> reveal with password -> hide -> verify mask | command_ref: CMD-VSD-BROWSER.
- AC-VSD-09 | layer: frontend | case: no secret leakage in mocks, screenshots, errors, or normal state | red: mock fixtures or browser artifacts contain sensitive plaintext | green: frontend check and evidence review show sensitive mock records use masks and artifacts exclude plaintext | interaction: inspect variable mock state and browser artifacts after reveal/hide flows | command_ref: CMD-FE-CHECK.
- AC-VSD-10 | layer: backend | case: non-sensitive regression and agent variable compatibility | red: legacy non-sensitive variables break list/mutation or agent invocation receives masks for ordinary values | green: backend tests prove non-sensitive values remain visible and runtime consumption still receives ordinary plaintext | command_ref: CMD-BE-VARIABLE-TEST.

## Command Index

- CMD-BE-CHECK | command: make be-check | cost: medium | when: green/verify/final | rerun: after backend source or dependency changes | expect: backend compiles through the project Makefile.
- CMD-BE-VARIABLE-TEST | command: make be-test | cost: medium | when: red/green/verify/final | rerun: after backend variable, auth, runtime, crypto, config, or migration changes | expect: backend tests pass including sensitive variable coverage.
- CMD-FE-CHECK | command: make fe-check | cost: medium | when: green/verify/final | rerun: after frontend variable API, model, page, mock, or test changes | expect: frontend typecheck and production build pass.
- CMD-VSD-BROWSER | command: make fe-test TEST='variable-management-sensitive.spec.ts --reporter=line' | cost: slow/browser | when: verify | rerun: review-if-stale or after variable page, mock, auth, or API contract changes | expect: concrete browser evidence for mask, reveal, hide, search, edit, and remask flows.
- CMD-DOCS-TASKS | command: python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/variable-sensitive-data --phase tasks | cost: low | when: docs | rerun: after task or spec edits | expect: AC, proof, and task rows validate.
- CMD-DOCS-WORKTREES | command: python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/variable-sensitive-data --phase worktrees | cost: low | when: dispatch | rerun: after worktree plan or task edits | expect: worktree plan validates before worker dispatch.
- CMD-DOCS-FINAL | command: python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/variable-sensitive-data --phase final | cost: low | when: final | rerun: after report, task, or orchestration edits | expect: feature docs validate end to end.

## Evidence Rules

- Every `AC-VSD-*` row must have final evidence in reports.
- Backend evidence must include RED failure and GREEN pass for storage, display, reveal, edit, runtime, and regression behavior.
- Browser evidence must list clicks, inputs, network assertions, visible feedback, and artifacts.
- Evidence must never include sensitive plaintext secret values.
- Slow browser evidence is reused during review unless missing, failed, stale, or explicitly requested.
