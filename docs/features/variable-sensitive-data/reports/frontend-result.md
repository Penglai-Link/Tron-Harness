# Frontend Result: Variable Sensitive Data

## Summary

- status: done
- scope: workspace variable masking and reveal, platform variable masking and edit behavior, leak-safe mock and artifact checks.
- conclusion: frontend AC rows passed browser and build verification.

## Evidence Rows

- AC-VSD-07 | status: done | red: browser test failed before workspace reveal hide and remask controls existed | green: workspace browser flow passed for mask search miss reveal hide refresh and reload remask | command_ref: CMD-VSD-BROWSER | browser: clicks=/variables search reveal hide refresh reload, inputs=secret-substring search and password step-up, network=step-up and workspace reveal calls asserted, feedback=mask reveal hide and remask visible, artifacts=workspace-hidden.png workspace-reload-remasked.png.
- AC-VSD-08 | status: done | red: browser test failed before platform sensitive toggle preserve replace and reveal behavior existed | green: platform browser flow passed for create sensitive edit preserve reveal hide replace and remask | command_ref: CMD-VSD-BROWSER | browser: clicks=/settings/variables create toggle save edit reveal hide replace, inputs=name description masked value replacement and password step-up, network=create preserve edit step-up platform reveal and replace calls asserted, feedback=mask preserved value reveal hide replace remask visible, artifacts=platform-hidden.png platform-replace-remasked.png.
- AC-VSD-09 | status: done | red: source check failed while the legacy mock fixture still carried a plaintext credential marker | green: mock fixture marker was removed and browser artifacts were captured only after hide or remask states | command_ref: CMD-FE-CHECK | browser: clicks=workspace reveal hide reload and platform reveal hide replace, inputs=password step-up and non-secret search token, network=step-up workspace reveal platform reveal create preserve and replace calls asserted, feedback=only masks hidden states and remasked states persisted in artifacts, artifacts=workspace-hidden.png workspace-reload-remasked.png platform-hidden.png platform-replace-remasked.png.

## Remediation Evidence

- AC-VSD-07 | status: done | red: review found workspace step-up verification lacked the reveal target and non-sensitive to sensitive edits could submit preserve semantics | green: workspace UI now sends target scope workspaceCode and variableKey, requires replacement value for non-sensitive to sensitive edits, and asserts `REPLACE` network payload | command_ref: CMD-VSD-BROWSER | browser: clicks=workspace reveal hide local variable sensitive toggle save, inputs=password and replacement value, network=target-bound step-up and workspace PATCH asserted, feedback=mask reveal hide and validation message visible.
- AC-VSD-08 | status: done | red: review found platform step-up verification lacked target binding and sensitive demotion could save an empty value | green: platform UI now sends target scope and variableKey, requires replacement value for sensitive demotion, and asserts platform PATCH replacement payload | command_ref: CMD-VSD-BROWSER | browser: clicks=platform reveal edit sensitivity save, inputs=password and demotion replacement value, network=target-bound step-up and platform PATCH asserted, feedback=validation message and remask visible.

## Changed Paths

- tron-frontend/src/domain/variable/api
- tron-frontend/src/domain/variable/mock
- tron-frontend/src/domain/variable/model
- tron-frontend/src/domain/variable/pages/CreatePlatformVariablePage.tsx
- tron-frontend/src/domain/variable/pages/EditPlatformVariablePage.tsx
- tron-frontend/src/domain/variable/pages/PlatformVariableDetailsPage.tsx
- tron-frontend/src/domain/variable/pages/PlatformVariablesPage.tsx
- tron-frontend/src/domain/variable/pages/VariableManagementPage.tsx
- tron-frontend/tests/variable-management-sensitive.spec.ts

## Commands

- CMD-VSD-BROWSER: `make fe-test TEST='variable-management-sensitive.spec.ts --reporter=line'` passed 3 tests.
- CMD-FE-CHECK: `make fe-check` passed typecheck and production build.
- CMD-FE-CHECK: `rg -n "sk-tron-core-demo|revealed-[a-z0-9_-]+-value|StepUpPass-For-Test" tron-frontend/test-results tron-frontend/src/domain/variable/mock/data.ts || true` returned no sensitive marker matches.

## Blockers

- None.
