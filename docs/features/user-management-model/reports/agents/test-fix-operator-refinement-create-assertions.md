# TEST-FIX-02: operator-refinement create assertions alignment

## Scope and result
- Scope: Worker `TEST-FIX-02` on branch `feature/clouds3n/user-management-model/test/fix-operator-refinement-create-assertions`.
- Scope target: fix retained smoke assertions for create flow to match EV-08 operator-refinement contract where `userCode` is backend-generated.
- Result: updated retained create assertions in `tron-frontend/tests/user-management-smoke.spec.ts`; all required commands passed.

## Exact retained tests updated
- `TDD-UM-10: add user flow form posts uniform create request and renders created user`
- `consistency refactor TDD-UM-18: Add User navigates to create page and submits from page owner`
- `toast feedback TDD-UM-19: Create User success renders shared top-right toast`
- `loading feedback TDD-UM-20: Create User submit disables locally while pending and resolves through toast`

## Command execution results
| Command | Status | Result |
| --- | --- | --- |
| `rtk make fe-check` | pass | typecheck + production build passed |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "TDD-UM-10"` | pass | 1/1 passed |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "TDD-UM-18"` | pass | 1/1 passed |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "TDD-UM-19"` | pass | 4/4 passed |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "TDD-UM-20"` | pass | 3/3 passed |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "TDD-UM-24"` | pass | 1/1 passed |
| `rtk git diff --check` | pass | no issues reported |

## EV-08 alignment note
- For all updated retained create tests, manual `userCode` is no longer required or filled.
- Each updated create flow now asserts the create page has no `/user code/` control (`toHaveCount(0)`), and validates create request body does not include `userCode`.
- Success assertions now include backend-generated `userCode` visibility (`USER_MAYA_SINGH`) after create success, preserving create-page success coverage while staying consistent with `TDD-UM-24`.
- TDD-UM-24 stayed unchanged and remains authoritative for create-page contract; retained tests were only aligned to, not relaxed.

## Acceptance / TDD mapping
- AC-UM-02
  - Retained create flow coverage: `TDD-UM-10`, `consistency refactor TDD-UM-18`, `TDD-UM-24`.
- AC-UM-08
  - Retained create + interaction evidence: `TDD-UM-10`, `consistency refactor TDD-UM-18`, `TDD-UM-19`, `TDD-UM-20`.
- AC-UM-10
  - Create request ownership and shared page interaction: `TDD-UM-18`, `TDD-UM-24`.
- AC-UM-12
  - Shared toast source contract retained as-is, with create success branch aligned: `TDD-UM-19`.
- AC-UM-13
  - Local create loading state and shared completion feedback retained as-is, with aligned payload contract: `TDD-UM-20`.
- TDD-UM-10
  - Add User opens create route, submits payload, and renders created user.
- TDD-UM-18
  - Create-page ownership and route/navigation assertions include no editable manual `userCode`.
- TDD-UM-19
  - Shared top-right toast branch for create confirms success feedback after create submit without userCode input.
- TDD-UM-20
  - Local loading + disabled submit state branch confirms create request omits userCode and resolves with shared toast.
- TDD-UM-24
  - Not modified; retained tests now explicitly align with its no-manual-code and backend-generated-code assertions.

## Residual risks / blockers
- No blockers found after command verification.
- Residual risk: if frontend field accessibility text for create `user code` changes from a label-based query, `toHaveCount(0)` for `/user code/i` should be adjusted to continue validating the EV-08 intent.
