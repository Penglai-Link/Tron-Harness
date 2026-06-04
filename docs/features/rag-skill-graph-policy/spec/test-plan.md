# Test Plan: RAG Skill Graph Policy

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-06-04

用户已授权使用 codex-superpower 完成实现。所有 embedding 和 rerank 相关验证使用 fake runtime；本地测试不访问公司模型网络。

## Proof Rows

- AC-RSGP-01 | layer: backend-integration | case: default seed omits related graph whitelists and accepts custom node kind and relationship | red: ingest rejects a valid custom relation | green: ingest/search/expansion succeeds | interaction: not applicable | command_ref: CMD-RSGP-FOCUSED.
- AC-RSGP-02 | layer: backend-integration | case: test updates profile with explicit whitelist and verifies out-of-list relation rejection | red: configured whitelist is ignored | green: ingest returns `RagError::IngestRequestInvalid` | interaction: not applicable | command_ref: CMD-RSGP-FOCUSED.
- AC-RSGP-03 | layer: static | case: search production RAG graph repository for removed default business constants | red: constants remain in policy code | green: constants appear only outside production graph repository write policy | interaction: not applicable | command_ref: CMD-RSGP-STATIC.
- AC-RSGP-04 | layer: backend-integration | case: dynamic `skillAlias` field added through corpus field mapping is written and returned | red: field is absent from graph hit fields | green: search returns `skillAlias` | interaction: not applicable | command_ref: CMD-RSGP-FOCUSED.
- AC-RSGP-05 | layer: backend-integration | case: edge scalar properties are persisted and returned by graph expansion | red: edge properties are empty or absent | green: properties match request payload | interaction: not applicable | command_ref: CMD-RSGP-FOCUSED.
- AC-RSGP-06 | layer: regression | case: Makefile backend suite covers existing RAG corpora and startup wiring | red: regression suite fails from this change | green: suite passes | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSGP-07 | layer: verification | case: focused tests use `FakeRuntime` and no provider configuration | red: test requires external model call | green: fake runtime counters are asserted | interaction: not applicable | command_ref: CMD-RSGP-FOCUSED.

## Command Index

- CMD-RSGP-FOCUSED | command: cargo test --manifest-path tron-backend/Cargo.toml --test rag_skill_graph_contracts -- --nocapture | cost: medium | when: red/green | rerun: after graph policy, field mapping, or edge property changes | expect: focused skill graph tests prove policy behavior.
- CMD-RSGP-STATIC | command: rg -n 'TOOL|CONFIG|VARIABLE|TAG|REQUIRES_TOOL|HAS_CONFIG|USES_VARIABLE|USED_BY_AGENT|HAS_TAG|corpus_policy_defaults' tron-backend/src/adapters/outbound/persistence/rag_graph_repository -S | cost: fast | when: verify | rerun: after graph repository policy edits | expect: no removed default write policy constants in production graph repository code.
- CMD-BE-CHECK | command: make be-check | cost: medium | when: verify | rerun: after backend module wiring or trait changes | expect: backend check passes.
- CMD-BE-TEST | command: make be-test | cost: high | when: final verify | rerun: after backend behavior or migration changes | expect: backend tests pass; model-dependent tests use fake runtimes.
- CMD-DOCS-FINAL | command: python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-skill-graph-policy --phase final | cost: fast | when: final docs | rerun: after docs, task, orchestration, or report changes | expect: docs validate.

## Evidence Rules

- RED evidence must precede GREEN for changed behavior.
- Any model-dependent proof must use fake runtime evidence.
- Browser evidence is not required because this feature has no frontend surface.
