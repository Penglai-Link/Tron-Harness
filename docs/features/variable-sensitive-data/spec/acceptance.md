# Acceptance: Variable Sensitive Data

## Acceptance Index

- AC-VSD-01 [must] [database] Variable tables persist sensitive metadata while legacy rows stay non-sensitive and compatible; proof: backend migration and repository tests pass.
- AC-VSD-02 [must] [security] Sensitive writes store SM4-GCM ciphertext without plaintext and decrypt only through backend-controlled paths; proof: backend tests pass.
- AC-VSD-03 [must] [api] Normal variable display APIs return masks and never search or expose sensitive plaintext; proof: backend API tests pass.
- AC-VSD-04 [must] [api] Sensitive edits preserve, replace, demote, and inherit according to the agreed safety rules; proof: backend API tests pass.
- AC-VSD-05 [must] [api] Step-up and reveal endpoints return one plaintext value only for a valid scoped token; proof: backend API tests pass.
- AC-VSD-06 [must] [security] Runtime preparation uses a plaintext resolver and fails untrusted service-mode secret delivery; proof: backend runtime tests pass.
- AC-VSD-07 [must] [interaction] Workspace variable UI masks, reveals, hides, and searches sensitive rows without plaintext leakage; proof: frontend and browser tests pass.
- AC-VSD-08 [must] [interaction] Platform variable UI masks, reveals, hides, and edits sensitive rows safely; proof: frontend and browser tests pass.
- AC-VSD-09 [must] [security] Logs, errors, mocks, screenshots, and normal frontend state contain no sensitive plaintext; proof: verification review passes.
- AC-VSD-10 [must] [regression] Existing non-sensitive variable management and agent variable consumption remain compatible; proof: regression tests pass.

## Open Decisions

- None.
