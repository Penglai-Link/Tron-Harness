# Feature Spec: RAG Pipeline DAG Runtime

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_user_supplied_design: yes
- drafted_from_repository_findings: yes
- user_confirmation_status: approved_by_user_request

本切片升级 RAG Profile 配置和运行时 Pipeline，使写入和检索不再由固定 UI/固定 route/stage 结构表达，而是由后端声明的 Stage、Strategy、artifact contract 和 DAG 拓扑驱动。

## Feature Identity And Context

- feature_slug: rag-pipeline-dag-runtime
- feature_name: RAG Pipeline DAG Runtime
- status: approved_from_user_supplied_design
- owner: langtaoxin
- source_request: 使用最新修正版方案升级 RAG Profile 配置和运行时 Pipeline，使 ingest/search API 保持兼容，同时让 Profile 管理、Stage/Strategy Registry、DAG 执行和前端配置页支持分叉、并行和汇聚。
- dependency_features: `rag-ingest-dag`, `rag-management-api-integration`, `rag-storage-runtime-workbench`, `rag-storage-write-planner`
- dependency_contracts: `tron-backend/src/application/rag/management.rs`, `tron-backend/src/application/rag/ingest_profile.rs`, `tron-backend/src/application/rag/search_profile.rs`, `tron-backend/src/application/rag/ingest.rs`, `tron-backend/src/application/rag/search.rs`, `tron-frontend/src/domain/rag/pages/RagProfileFormPage.tsx`, `tron-frontend/src/domain/rag/components/RagWorkbenchTab.tsx`

## Requirement

用户需要 RAG 写入和检索阶段流程不固定，Profile 根据业务需求选择阶段和策略。阶段之间通过统一 artifact map 传递数据，新增阶段不应改变运行时 ingest/search 请求结构，也不应要求修改上下文固定字段。

## Product Source Slices

- user_design_source: `/Users/langtaoxin/.codex/attachments/42a0a4f8-514c-4d7a-83e3-603c0adbbb15/pasted-text.txt`
- source_sections: `核心原则`, `运行时 API 不变`, `Profile Pipeline 配置`, `Stage 和 Strategy 职责`, `Stage Catalog`, `DAG Runtime`, `前端 Profile 页面`, `Workbench`
- repository_context: RAG management API, ingest runtime, retrieval runtime, profile form pages, and workbench page.

## Scope Boundary

### Assumptions

- Runtime ingest/search request structures remain stable.
- Runtime responses may add optional fields but cannot remove or rename old fields.
- Management profile payload does not need old structure compatibility.
- Stage owns artifact input and output keys.
- Strategy cannot override artifact keys.
- Ingest and retrieval contexts are separate.
- The first UI version can use tables and selects instead of a visual graph editor.

### In Scope

- Pipeline Catalog Registry for supported Stage and Strategy declarations.
- `GET /api/v1/rag/pipeline/catalog`.
- Profile management payload with `pipeline.mode`, `pipeline.nodes`, and `pipeline.edges`.
- DAG validation for node uniqueness, edge references, cycles, profile type, strategy ownership, config, and policy values.
- Ingest and retrieval runtime stage results as additive response fields.
- Artifact map context shell for ingest and retrieval.
- Minimal DAG executor semantics for branch, ready set, and merge policy.
- Frontend Profile Pipeline Builder driven by catalog data.
- Workbench stage result rendering.

### Out Of Scope

- No runtime request breaking changes.
- No deletion of old runtime response fields.
- No new storage binding expansion beyond this feature's needs.
- No diagnostics UI redesign.
- No full visual graph canvas editor.
- No real implementation of future advanced strategies such as question rewriting unless an existing backend strategy already supports it.

### Constraints

- Use Makefile first for project checks.
- API responses must use `{ code, message, data }`.
- New management endpoints should follow GET/POST and kebab-case URL rules where practical.
- Existing seeded profiles must not make runtime startup fail.
- Frontend mock mode must stay aligned with backend catalog shape.

## Feature Model

Profile executable configuration is represented by one DAG model:

```json
{
  "pipeline": {
    "mode": "DAG",
    "nodes": [
      {
        "nodeId": "prepare-payload",
        "stageType": "PREPARE_PAYLOAD",
        "strategyId": "RAG_TEXT_CONTRACT_PAYLOAD_V1",
        "enabled": true,
        "joinPolicy": "ALL_SUCCESS",
        "failurePolicy": "FAIL_PIPELINE",
        "config": {}
      }
    ],
    "edges": [
      { "from": "prepare-payload", "to": "split-text" }
    ]
  }
}
```

Stage declaration owns profile type, input artifacts, output artifacts, repeatability, parallel support, default join policy, default failure policy, and allowed strategies.

Strategy declaration owns strategy id, display metadata, config schema, required capabilities, required storage segments, and executor implementation.

Artifact data is stored in `artifacts` maps inside separate ingest and retrieval contexts. Context shells carry request-level data and diagnostics but do not grow fixed fields for every new stage.

## Page And Module Surface

### Backend API

- `GET /api/v1/rag/pipeline/catalog`
- `POST /api/v1/rag/profiles`
- `POST /api/v1/rag/profiles/update`
- `POST /api/v1/rag/profiles/verify`
- `POST /api/v1/rag/ingest`
- `POST /api/v1/rag/search`

### Backend Modules

- RAG management service validates and persists new profile pipeline payloads.
- Pipeline catalog module exposes supported stage and strategy declarations.
- Pipeline compiler validates DAG topology and artifact contracts.
- Runtime ingest/search services emit additive stage results.
- Search trace persistence records safe stage execution summaries for trace-enabled retrieval requests.

### Frontend Pages

- RAG ingest profile create/edit.
- RAG retrieval profile create/edit.
- RAG Workbench.

## Repository Findings

- Phase 1-3 established Pipeline DAG management, generic ingest/retrieval runtime execution, advanced stages, and additive runtime `stageResults`.
- Phase 4 closes the observability loop by persisting safe search trace stage summaries when trace is enabled.
- Phase 4 also refreshes Workbench evidence so users can inspect stage execution rows from actual runtime responses and still use legacy responses without `stageResults`.

## Open Decisions

- None.
