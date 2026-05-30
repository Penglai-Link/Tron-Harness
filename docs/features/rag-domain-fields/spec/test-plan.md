# Test Plan: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-05-30

本测试计划按用户授权直接通过。验证重点放在 RAG domain fields 的 schema evolution、seed projection、filterSchema 校验和边界回归，不覆盖真实 ingest、真实 search、HTTP、embedding、GraphService 或 EventBus 行为。

## Proof Rows

- AC-RDF-01 | layer: backend-integration | case: SQLite migration adds Memory governance columns and indexes idempotently | red: schema test fails before domain fields migration exists | green: schema test passes after Memory fields are added | command_ref: CMD-BE-TEST.
- AC-RDF-02 | layer: backend-integration | case: SQLite migration adds Project Doc repo/module columns and indexes idempotently | red: schema test fails before Project Doc domain fields exist | green: schema test passes after Project Doc fields are added | command_ref: CMD-BE-TEST.
- AC-RDF-03 | layer: backend-unit | case: `memory-card` fieldMapping and filterSchema expose Memory logical fields | red: service test fails when Memory fields are absent from seed projection | green: service test passes with Memory logical fields mapped | command_ref: CMD-BE-TEST.
- AC-RDF-04 | layer: backend-unit | case: `project-doc` fieldMapping and filterSchema expose Project Doc logical fields | red: service test fails when Project Doc fields are absent from seed projection | green: service test passes with Project Doc logical fields mapped | command_ref: CMD-BE-TEST.
- AC-RDF-05 | layer: backend-unit | case: filter validation accepts declared Memory and Project Doc fields and rejects invalid values | red: filter validation test accepts unknown or mistyped domain fields | green: filter validation test returns normalized filters or field-level errors | command_ref: CMD-BE-TEST.
- AC-RDF-06 | layer: backend-regression | case: boundary tests confirm no ingest/search route or forbidden dependency behavior appears | red: boundary test fails when HTTP route or model/graph/event/Memory flow coupling appears | green: compile and boundary tests pass without forbidden integrations | command_ref: CMD-BE-CHECK.
- AC-RDF-06 | layer: docs-validation | case: feature documentation remains AC-only and progressive spec gates pass | red: docs validation fails when invalid trace IDs or downstream artifacts appear | green: docs validation passes for complete spec gate | command_ref: CMD-DOCS-SPEC.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after RAG migration, repository, service, or test changes | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring or dependency boundary changes | expect: backend compile check passes.
- CMD-DOCS-SPEC | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-domain-fields --phase spec --spec-stage test-plan --complete-spec --no-downstream-artifacts` | cost: fast | when: verify | rerun: after spec document changes | expect: complete spec validation passes.

## Evidence Rules

- RED evidence must show the failing unit, integration, compile, or docs validation command before implementation changes.
- GREEN evidence must show the matching command passing after implementation.
- Browser evidence is not required because this feature has no frontend or HTTP route surface.
- Slow external services are not required because this feature does not call model providers, GraphService, or EventBus.
