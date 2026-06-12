# Frontend Agent Report: AC-RPD Pipeline Builder

## Scope

- AC-RPD-06: Profile form pipeline builder surfaces and mutation payload.
- AC-RPD-07: Workbench `stageResults` rendering and fallback summary.
- AC-RPD-08: Frontend runtime request compatibility evidence.

## RED

- command: `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts`
- result: failed after Playwright assertions were updated.
- summary: Profile forms did not expose `Pipeline Nodes`/`Pipeline Edges`; Workbench did not expose `Ingest Stage Results`.

## GREEN

- command: `make fe-test TEST=tron-frontend/tests/rag-profile-form-pages.spec.ts`
- result: passed, 3 tests.
- summary: Ingest and retrieval profile forms now save `pipeline.mode/nodes/edges`; Workbench displays stage execution rows and preserves fallback behavior when no `stageResults` exist.

## Browser Evidence

- AC-RPD-06 | clicks=Add Ingest Profile, Save Profile, Edit support-notes-ingest-v1, Add Retrieval Profile, Add Pipeline Node, Add Filter, Edit support-fusion-v1 | inputs=profile IDs, corpus selects, descriptions, node toggles, stage type/strategy selects, source type text, filter rows, topic policy | network=profile list/catalog/detail plus verify/create/update requests asserted; create/update bodies include `pipeline` and omit old `pipelineStages`/`routes` | feedback=Pipeline Nodes, Pipeline Edges, Stage Config, Backend Payload Preview, Verify Result, created/saved toasts | artifacts=Playwright stdout, test file assertions.
- AC-RPD-07 | clicks=Workbench tab, Submit Ingest, Add Query Field, Add Filter Condition, Run Search | inputs=source metadata, business attributes, text content, query, return fields, filter, limit, token budget | network=ingest/search request bodies asserted unchanged; responses mocked with and without `stageResults` | feedback=Ingest Stage Results, Search Stage Results, legacy fallback summary and result cards | artifacts=Playwright stdout, test file assertions.

## VERIFY

- command: `make fe-check`
- result: passed typecheck and production build.

## Changed Paths

- `tron-frontend/src/domain/rag/api/ragManagementApi.ts`
- `tron-frontend/src/domain/rag/components/RagWorkbenchTab.tsx`
- `tron-frontend/src/domain/rag/model/RagModel.ts`
- `tron-frontend/src/domain/rag/model/ragProfileForm.ts`
- `tron-frontend/src/domain/rag/pages/RagProfileFormPage.tsx`
- `tron-frontend/tests/rag-management-api-mock.ts`
- `tron-frontend/tests/rag-profile-form-pages.spec.ts`

## Blockers

- none.
