# Test Plan: RAG 后端底座 - Corpus 存储与 Profile 基础

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-05-30

本测试计划按用户授权直接通过。验证重点放在 RAG foundation 的 schema、seed、只读 service、Profile 校验、filterSchema 校验和边界回归，不覆盖真实 ingest、真实 search、debug HTTP、embedding、GraphService 或 EventBus 行为。

## Proof Rows

- AC-RAG-01 | layer: backend-integration | case: SQLite migration creates all approved tables and seeds three Corpus rows idempotently | red: migration test fails before RAG foundation migration exists | green: migration test passes after schema and seed are implemented | command_ref: CMD-BE-TEST.
- AC-RAG-02 | layer: backend-integration | case: schema inspection confirms per-Corpus Source/TextUnit tables and merged fulltext/vector TextUnit fields | red: schema test fails when unified Source/TextUnit or separate vector tables appear | green: schema test passes with the approved 7 table layout | command_ref: CMD-BE-TEST.
- AC-RAG-03 | layer: backend-unit | case: Corpus binding resolves Source, TextUnit, diagnostics, fieldMapping, and filterSchema for enabled Corpus | red: service test fails when binding JSON is missing or points to wrong tables | green: service test passes with seeded binding records | command_ref: CMD-BE-TEST.
- AC-RAG-04 | layer: backend-unit | case: `RagService` lists and reads Corpus/Profile DTOs through repository ports | red: service test fails when callers depend on SQLite-specific records | green: service test passes with port-based DTO projection | command_ref: CMD-BE-TEST.
- AC-RAG-05 | layer: backend-unit | case: Retrieval Profile validation rejects missing or `latest` source version values | red: profile validation test accepts missing or latest versions | green: profile validation test accepts only concrete versions | command_ref: CMD-BE-TEST.
- AC-RAG-06 | layer: backend-unit | case: strategy registry validation reports enabled, reserved, and unsupported strategy references | red: registry test fails when unknown or reserved references pass silently | green: registry test returns expected validation errors | command_ref: CMD-BE-TEST.
- AC-RAG-07 | layer: backend-unit | case: filterSchema validation accepts declared fields and rejects unknown fields, invalid types, and unsupported operators | red: filter validation test accepts invalid filters | green: filter validation test returns field-level errors | command_ref: CMD-BE-TEST.
- AC-RAG-08 | layer: backend-integration | case: schema and model tests preserve `scope_type` and `scope_code` without permission-side effects | red: schema/model test fails when scope columns are missing or permission logic appears | green: schema/model test passes with isolation fields only | command_ref: CMD-BE-TEST.
- AC-RAG-09 | layer: backend-regression | case: compile and boundary tests confirm no RAG HTTP routes or model, graph, event calls are introduced | red: boundary test fails when route registration or forbidden dependency call appears | green: compile and boundary tests pass without forbidden integrations | command_ref: CMD-BE-CHECK.
- AC-RAG-09 | layer: docs-validation | case: feature documentation remains AC-only and progressive spec gates pass | red: docs validation fails when downstream artifacts or invalid trace IDs appear | green: docs validation passes for complete spec gate | command_ref: CMD-DOCS-SPEC.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after backend domain, migration, repository, or service changes | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring or dependency boundary changes | expect: backend compile check passes.
- CMD-DOCS-SPEC | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-corpus-storage-profile-foundation --phase spec --spec-stage test-plan --complete-spec --no-downstream-artifacts` | cost: fast | when: verify | rerun: after spec document changes | expect: complete spec validation passes.

## Evidence Rules

- RED evidence must show the failing unit, integration, compile, or docs validation command before implementation changes.
- GREEN evidence must show the matching command passing after the implementation change.
- Browser evidence is not required because this feature has no frontend or HTTP route surface.
- Slow external services are not required because this feature does not call model providers, GraphService, or EventBus.
