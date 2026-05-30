# Review Report: RAG 后端底座 - Corpus 存储与 Profile 基础

## Summary

Lead review 已完成。审查范围包括 AC 覆盖、7 表边界、只读 RAG 配置服务、Profile 校验、filterSchema 校验、scope 字段、禁止 HTTP/model/graph/event 集成，以及 superpower 文档闭环。

## Evidence Rows

- AC-RAG-01 | red: review 对照数据库 worker RED 证据确认实现前缺少 RAG foundation migration | green: review 确认 migration 创建 7 张批准表并幂等 seed `memory-card`、`project-doc`、reserved `code-graph` | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-02 | red: review 对照 schema 检查确认实现前没有可验证的分表与合并索引字段结构 | green: review 确认 Source/TextUnit 按 Corpus 分表，全文和向量字段在各 Corpus TextUnit 表内 | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-03 | red: review 对照服务测试确认实现前没有 CorpusTableBinding 投影 | green: review 确认 enabled Corpus 可读取 Source/TextUnit/diagnostics 表映射、fieldMapping 和 filterSchema | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-04 | red: review 对照服务测试确认实现前没有 repository port 与 DTO 投影 | green: review 确认 `RagService` 通过 port 列表和读取 Corpus/Profile，不把 SQLite record 暴露给应用调用方 | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-05 | red: review 对照 Profile 校验测试确认实现前未拒绝缺失版本和 `latest` | green: review 确认 Retrieval Profile 必须配置具体 `requiredSourceVersion`，运行语义不使用 `latest` | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-06 | red: review 对照 strategy registry 测试确认实现前不能报告 enabled、reserved、unsupported 差异 | green: review 确认 enabled Profile 拒绝 reserved 策略，reserved Profile 可保留 reserved 策略引用 | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-07 | red: review 对照 filterSchema 测试确认实现前无字段级校验 | green: review 确认只接受声明的逻辑字段，并拒绝未知字段、类型错误和不支持操作符 | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-08 | red: review 对照 schema/model 测试确认实现前缺少 scope 字段证据 | green: review 确认仅保留 `scope_type` 和 `scope_code` 数据隔离字段，没有实现权限业务逻辑 | command_ref: none | browser:not applicable | status: passed.
- AC-RAG-09 | red: review 对照边界测试确认实现前没有防止误接入的回归保护 | green: review 确认未新增 RAG HTTP route，也未调用模型服务、graph 服务或事件总线 | command_ref: none | browser:not applicable | status: passed.

## Changed Paths

- `docs/features/rag-corpus-storage-profile-foundation/reports/database-result.md`
- `docs/features/rag-corpus-storage-profile-foundation/reports/backend-result.md`
- `docs/features/rag-corpus-storage-profile-foundation/reports/review-report.md`
- `docs/features/rag-corpus-storage-profile-foundation/orchestration/integration-log.md`
- `docs/features/rag-corpus-storage-profile-foundation/orchestration/worktree-plan.md`
- `docs/features/rag-corpus-storage-profile-foundation/orchestration/feature-evolution-log.md`
- `docs/features/rag-corpus-storage-profile-foundation/tasks/database-task.md`
- `docs/features/rag-corpus-storage-profile-foundation/tasks/backend-task.md`
- `docs/features/rag-corpus-storage-profile-foundation/tasks/review-task.md`

## Commands

- `make be-test`: passed after backend worker merge.
- `make be-check`: passed after backend worker merge.
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-corpus-storage-profile-foundation --phase final`: passed.

## Blockers

- None.
