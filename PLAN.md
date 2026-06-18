**Skills RAG Indexing Plan**

**Summary**
- Keep the existing RAG public entry as `application::rag::RagService`; do not let `skills` write SQLite/vector/graph storage directly.
- Add a skill-specific indexing service that converts enabled workspace skills into two existing RAG corpora:
  - `skill`: vector/fulltext index containing only `skillName` and `skillDescription`.
  - `skill-graph`: graph-backed skill node plus `CONTAINS` edges to structure files.
- Scope indexing to effective enabled workspace skills only. Disable/unbind/uninstall must mark old RAG sources inactive.

**Key Changes**
- Add `SkillRagIndexService` under `application::skill` that:
  - Reads effective workspace skills through `SkillRegistryService`.
  - Reads `SKILL.md` and file list through existing skill file helpers.
  - Builds stable source identity: `sourceCode = skill:{workspaceCode}:{skillName}`, `sourceVersion = v1`, `scope = SPACE/{workspaceCode}`.
  - Calls `RagService::ingest` twice on enable/update/refresh: once for `skill`, once for `skill-graph`.
- Build vector payload text as:
  - `Skill: {skillName}`
  - `Description: {frontmatterDescription}`
  - Store `skillName` and `skillDescription` in `domainFields`.
- Build graph payload with:
  - Main skill node from the same name/description text.
  - `relatedGraph.nodes` for all files returned by `list_skill_files`, classified by path: `SKILL_MD`, `SCRIPT`, `REFERENCE`, `OTHER`.
  - `relatedGraph.edges` from `$skill` to each file node with relationship `CONTAINS`.
  - No `USE` edges in v1, per decision; leave the shape open for later deterministic dependency parsing.
- Add a RAG source lifecycle path:
  - New application repository port to deactivate a source by `corpusId/sourceCode/sourceVersion/scope`.
  - SQLite implementation marks source/text units non-active and removes FTS/vector index rows.
  - Graph implementation marks the main graph skill node non-active so search filters exclude it.
- Add replace semantics for skill structure graph:
  - For `CONTAINS`, reindexing a skill removes old outgoing skill-local `CONTAINS` edges before writing the new file graph, preventing stale file structure.
- Add persistent skill RAG index state in skill storage:
  - Track `workspaceCode`, `skillName`, `sourceVersion`, `desiredEnabled`, `vectorStatus`, `graphStatus`, `contentHash`, `structureHash`, `lastIndexedAt`, `lastError`, `updatedAt`.
  - Status values: `PENDING`, `INDEXED`, `DISABLED`, `WARNING`, `FAILED`, `STALE`.
  - Mutating endpoints first try synchronous indexing; if indexing fails, the skill operation still returns success with warning and persisted failure state.
  - Later `refreshWorkspaceSkills`, bind/enable, install-with-enable, registry update/rescan retry stale or failed index states.

**API And UI Surface**
- Keep existing RAG HTTP endpoints unchanged.
- Extend skill responses with optional `ragIndex` summary:
  - workspace mutation responses include per-skill vector/graph status and warnings.
  - registry sync/update responses include aggregate indexed/disabled/failed counts.
  - workspace skill list/detail include latest persisted `ragIndex` state.
- Update frontend skill models to accept optional `ragIndex`; show warning toast when mutation succeeds but index status is `WARNING` or `FAILED`.

**Test Plan**
- Add backend contract tests for:
  - enabling a workspace skill writes `skill` vector rows with only name/description fields.
  - enabling a workspace skill writes `skill-graph` node plus `CONTAINS` edges for `SKILL.md`, `scripts/*`, `references/*`, and other files.
  - disabling/unbinding marks both corpora inactive and search no longer returns the skill.
  - reindexing after file removal removes stale `CONTAINS` edges.
  - failed RAG indexing persists `FAILED` state while skill mutation still succeeds.
  - refresh retries a previously failed/stale index.
- Run verification through Makefile first: `make be-test`; if frontend types are touched, also run `make fe-check`.

**Assumptions**
- v1 indexes only effective enabled workspace skills, not all registry skills.
- v1 graph stores only `CONTAINS` structure edges; `USE` edges for other skills and environment variables are deferred.
- Missing frontmatter description is stored as an empty description and flagged in index warnings, rather than blocking indexing.
- No background worker is introduced in this slice; persisted index state is designed so a later async runner can consume it.
