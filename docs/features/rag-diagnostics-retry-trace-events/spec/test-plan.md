# Test Plan: RAG 后端底座 - 诊断、重试、Trace 与事件

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-05-31

本测试计划按用户授权直接通过。验证重点放在 debug API、retry API、warnings/trace 持久化、EventBus safe payload、workspace/scope 边界和回归边界。所有模型调用使用 fake runtime，不访问真实 provider。

## Proof Rows

- AC-RDT-01 | layer: backend-http | case: debug route validates query, auth user, workspace guard and direct scope | red: HTTP contract test accepts malformed, cross-user, no workspace access or wrong scope debug request | green: route returns uniform success or stable RAG debug error response | command_ref: CMD-BE-TEST.
- AC-RDT-02 | layer: backend-unit | case: debug service returns Profile, Source, TextUnit summary and warnings without forbidden payload | red: debug test leaks embedding blob or misses persisted warning | green: debug result includes summaries and excludes forbidden fields | command_ref: CMD-BE-TEST.
- AC-RDT-03 | layer: backend-http | case: retry API validates stages, source existence, text unit existence and no parse/normalize/split execution | red: retry test accepts unsupported stage or missing text units | green: retry rejects invalid requests and only calls index stages | command_ref: CMD-BE-TEST.
- AC-RDT-04 | layer: backend-integration | case: fulltext retry rebuilds search_text/fulltext_status from persisted TextUnits | red: SQLite test leaves failed fulltext rows unchanged | green: retry updates rows to INDEXED with rebuilt search_text and safe stage result | command_ref: CMD-BE-TEST.
- AC-RDT-05 | layer: backend-integration | case: vector retry uses fake LlmRuntimeService embedding and persists safe warnings on failure | red: retry stores provider raw response or fails to update embedding status | green: success updates embedding fields and failure writes safe INDEXING_WARNING | command_ref: CMD-BE-TEST.
- AC-RDT-06 | layer: backend-integration | case: search trace writes SEARCH_TRACE only when enabled and sets 7-day TTL | red: trace disabled still writes row or enabled trace stores body/embedding | green: enabled trace writes lightweight row with query_text, counts, refs, usage and expires_at | command_ref: CMD-BE-TEST.
- AC-RDT-07 | layer: backend-unit | case: RAG lifecycle EventBus events use safe payload and publish failure is non-blocking | red: event test finds body/embedding/provider payload or main flow fails on publish error | green: recording sink observes safe events and disabled/failing bus does not break operation | command_ref: CMD-BE-TEST.
- AC-RDT-08 | layer: backend-regression | case: source scan confirms no SummaryTree, GraphService, GRAPH route, code-graph execution, Profile UI or Memory flow behavior | red: boundary test finds forbidden dependency or executable reserved behavior | green: compile and boundary tests pass | command_ref: CMD-BE-CHECK.
- AC-RDT-08 | layer: docs-validation | case: feature documentation remains AC-only and final validation passes | red: docs validation fails on missing evidence rows or invalid trace IDs | green: final feature docs validation passes | command_ref: CMD-DOCS-FINAL.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after RAG debug, retry, trace, event, repository, HTTP or test changes | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring or dependency boundary changes | expect: backend compile check passes.
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-diagnostics-retry-trace-events --phase final` | cost: fast | when: verify | rerun: after feature docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence must show the failing unit, integration, HTTP, compile, or docs validation command before implementation changes.
- GREEN evidence must show the matching command passing after implementation.
- HTTP evidence must include request shape, response code/message/data, and negative cross-scope behavior.
- Embedding retry tests must use fake runtime; real provider calls are not acceptable evidence.
- Event evidence must include observed event names and forbidden payload assertions.
- Browser evidence is not required because this feature has no frontend surface.
