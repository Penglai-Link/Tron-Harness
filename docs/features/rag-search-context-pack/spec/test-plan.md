# Test Plan: RAG 后端底座 - 检索编排与 Context Pack

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-05-31

本测试计划按用户授权直接通过。验证重点放在 search API、Retrieval Profile 编译、filter 合并、fulltext/vector routes、hydrate、fusion/dedup/rerank、context assembly 和边界回归。所有模型调用使用 fake runtime，不访问真实 provider。

## Proof Rows

- AC-RSC-01 | layer: backend-http | case: search route validates request, auth user, workspace guard, and scope | red: HTTP contract test accepts malformed or cross-scope search request | green: route returns uniform success or stable error response | command_ref: CMD-BE-TEST.
- AC-RSC-02 | layer: backend-unit | case: Retrieval Profile compiler validates sourceVersion, routes, context and reserved retrievers | red: compiler test accepts invalid retrieval config | green: compiler rejects invalid configs and accepts seeded profiles | command_ref: CMD-BE-TEST.
- AC-RSC-03 | layer: backend-unit | case: Profile filters and Request filters merge by filterSchema | red: filter test allows unknown fields or broadens Profile filter | green: filter test rejects invalid fields and returns safe intersection | command_ref: CMD-BE-TEST.
- AC-RSC-04 | layer: backend-integration | case: memory-card fulltext/vector routes return scoped active CandidateHit rows | red: SQLite test returns inactive, wrong scope, or wrong version rows | green: search observes only active scoped hits | command_ref: CMD-BE-TEST.
- AC-RSC-05 | layer: backend-integration | case: project-doc search hydrates Source/TextUnit with heading/path metadata | red: hydrate test misses document fields or includes inactive source | green: result contains document metadata from active Source/TextUnit | command_ref: CMD-BE-TEST.
- AC-RSC-06 | layer: backend-unit | case: fusion, dedup, and fake rerank produce deterministic ordering | red: duplicate route hits leak or ordering is unstable | green: merged result keeps one hit per dedupKey with expected order | command_ref: CMD-BE-TEST.
- AC-RSC-07 | layer: backend-unit | case: context assembly supports result list and three context pack strategies | red: context test exceeds tokenBudget or emits wrong strategy shape | green: all context strategies respect limits and shape | command_ref: CMD-BE-TEST.
- AC-RSC-08 | layer: backend-regression | case: source scan confirms no debug/retry/trace/SummaryTree/GraphService/Memory flow behavior | red: boundary test finds forbidden route, trace write, or graph dependency | green: compile and boundary tests pass | command_ref: CMD-BE-CHECK.
- AC-RSC-08 | layer: docs-validation | case: feature documentation remains AC-only and final validation passes | red: docs validation fails on missing evidence rows or invalid trace IDs | green: final feature docs validation passes | command_ref: CMD-DOCS-FINAL.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after RAG search service, HTTP, repository, profile seed, or test changes | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring or dependency boundary changes | expect: backend compile check passes.
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-search-context-pack --phase final` | cost: fast | when: verify | rerun: after feature docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence must show the failing unit, integration, HTTP, compile, or docs validation command before implementation changes.
- GREEN evidence must show the matching command passing after implementation.
- HTTP evidence must include request shape, response code/message/data, and negative cross-scope behavior.
- Embedding and rerank tests must use fake runtime; real provider calls are not acceptable evidence.
- Browser evidence is not required because this feature has no frontend surface.
