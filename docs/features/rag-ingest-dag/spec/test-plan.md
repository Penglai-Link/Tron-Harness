# Test Plan: RAG 后端底座 - 同步写入 DAG

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-05-31

本测试计划按用户授权直接通过。验证重点放在 ingest API、DAG 编译、SQLite 持久化、embedding 降级、幂等写入和边界回归。所有模型调用使用 fake runtime，不访问真实 provider。

## Proof Rows

- AC-RID-01 | layer: backend-http | case: ingest route validates request, auth workspace guard, and target scope | red: HTTP contract test accepts malformed or cross-scope request | green: route returns uniform success or stable error response | command_ref: CMD-BE-TEST.
- AC-RID-02 | layer: backend-unit | case: Ingest Profile DAG compiler validates corpus, cycles, stages, artifact compatibility, persist stage | red: compiler test accepts invalid DAG | green: compiler test rejects invalid configs and accepts seeded profiles | command_ref: CMD-BE-TEST.
- AC-RID-03 | layer: backend-integration | case: memory-card plain text ingest writes Source, one TextUnit, fulltext field, and embedding fields | red: repository test finds no persisted Memory rows | green: SQLite test observes active Source and indexed TextUnit | command_ref: CMD-BE-TEST.
- AC-RID-04 | layer: backend-integration | case: project-doc Markdown ingest writes Source fields and split TextUnits | red: repository test finds missing Project Doc fields or only one unsplit unit | green: SQLite test observes document fields and heading-based TextUnits | command_ref: CMD-BE-TEST.
- AC-RID-05 | layer: backend-integration | case: repeated same version write updates rows without duplicates and second version coexists | red: idempotency test duplicates Source or TextUnit rows | green: repository row counts and content hashes match expected state | command_ref: CMD-BE-TEST.
- AC-RID-06 | layer: backend-unit | case: fake embedding failure follows Memory fail-fast and Project Doc warning degrade policies | red: failure test marks wrong Source status or loses warning | green: service test observes failed Memory and active Project Doc warning | command_ref: CMD-BE-TEST.
- AC-RID-07 | layer: backend-regression | case: source scan confirms no search/debug/retry/SummaryTree/Graph/Memory flow behavior | red: boundary test finds forbidden route or dependency | green: compile and boundary tests pass | command_ref: CMD-BE-CHECK.
- AC-RID-07 | layer: docs-validation | case: feature documentation remains AC-only and final validation passes | red: docs validation fails on missing evidence rows or invalid trace IDs | green: final feature docs validation passes | command_ref: CMD-DOCS-FINAL.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after RAG ingest service, HTTP, repository, migration, or test changes | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring or dependency boundary changes | expect: backend compile check passes.
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-ingest-dag --phase final` | cost: fast | when: verify | rerun: after feature docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence must show the failing unit, integration, HTTP, compile, or docs validation command before implementation changes.
- GREEN evidence must show the matching command passing after implementation.
- HTTP evidence must include request shape, response code/message/data, and negative cross-scope behavior.
- Embedding tests must use fake runtime; real provider calls are not acceptable evidence.
- Browser evidence is not required because this feature has no frontend surface.
