# Test Plan: RAG Skill Graph Corpus

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-06-04

用户已授权使用 codex-superpower 完成实现。所有 embedding 和 rerank 相关验证必须使用 fake runtime；本地测试不访问公司模型网络。

## Proof Rows

- AC-RSG-01 | layer: regression | case: existing skill, memory-card, and project-doc storage/search configs remain executable | red: regression test detects changed default corpus behavior | green: same regression passes after skill-graph is additive | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-02 | layer: database | case: migrations seed skill-graph corpus and graph profiles with graph single-node binding | red: schema/config test cannot load skill-graph records | green: migration test loads enabled corpus and profiles | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-03 | layer: backend-integration | case: skill-graph ingest writes exactly one RagSkill graph node with RAG embedding payload | red: graph node is missing or embedding is recomputed/unavailable | green: graph node has expected fields and fake embedding payload | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-04 | layer: backend-integration | case: relatedGraph payload validates policy and upserts related nodes and edges | red: disallowed or duplicate relation writes incorrectly succeed | green: whitelist, cap, idempotency, and rejection cases pass | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-05 | layer: backend-integration | case: skill-graph fulltext/vector search returns scoped CandidateHit rows | red: search route cannot read graph or leaks wrong scope | green: graph route rows enter existing ranking with expected scores | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-06 | layer: backend-unit | case: fusion dedup key includes corpus, source, version, scope, and target | red: equal targetCode across corpora or scopes collapses incorrectly | green: distinct identity rows survive dedup while true duplicates merge | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-07 | layer: backend-unit | case: relation expansion executes after rerank and final limit for graph hits only | red: non-final or non-graph candidates are expanded | green: only final skill-graph hits receive relatedGraph metadata | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-08 | layer: backend-integration | case: debug and retry route graph layout without SQLite table-plan failures | red: graph source debug/retry falls into SQLite table planner | green: debug returns graph node state and retry returns supported result or typed unsupported | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-09 | layer: regression | case: model-dependent tests use fake embedding/rerank runtimes | red: test path requires external provider configuration or network | green: fake runtime assertions prove no provider network dependency | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RSG-09 | layer: docs-validation | case: feature docs, tasks, worktree plan, and reports validate | red: feature docs validation fails before artifacts are complete | green: final feature docs validation passes | interaction: not applicable | command_ref: CMD-DOCS-FINAL.

## Command Index

- CMD-BE-TEST | command: make be-test | cost: medium | when: red/green/verify | rerun: after backend behavior, migration, or test changes | expect: backend tests pass or known unrelated failure is recorded with focused passing evidence.
- CMD-BE-CHECK | command: make be-check | cost: medium | when: verify | rerun: after backend module wiring or trait changes | expect: backend compile check passes or known unrelated architecture guard is recorded.
- CMD-DOCS-FINAL | command: python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-skill-graph-corpus --phase final | cost: fast | when: final | rerun: after docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence must be captured before implementing each behavior slice or must record why the command cannot isolate a pre-existing failure.
- GREEN evidence must use the same proof surface as RED.
- Any embedding or rerank proof must name the fake runtime path or assertion used.
- Browser evidence is not required because this feature has no frontend surface.
