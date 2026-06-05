# Context Index

Generated navigation index. Agents read this first, then open only the needed file, heading, and AC rows.

| file | kind | owns | headings | source_refs | ac_refs |
| --- | --- | --- | --- | --- | --- |
| spec/acceptance.md | acceptance-index | AC-* | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06, AC-MDPQ-07, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| spec/api-contract.md | spec-support | supporting facts | memory_dreaming.py task-detail, memory_dreaming.py task-complete, memory_dreaming.py finalize, memory_dreaming.py cleanup-temp, memory_dreaming_quality.py evaluate, memory_dreaming_live_smoke.py | none | none |
| spec/database-design.md | spec-support | supporting facts | Provenance Envelope, EvidenceRef, Light Memory Points, Summary Artifacts, REM And Deep Governance, Semantic Manifest | none | none |
| spec/feature-spec.md | feature-model | feature context | Assumptions, In Scope, Out Of Scope, Constraints | none | none |
| spec/source-coverage-audit.md | source-analysis | local audit rows mapped to FLOW/QUALITY/GOV/LTM additions | Source Aliases, Thickening Backlog, Release Rule | app-cisdigital-ai-intent-forge original design files, user supplemental requirement U1 | AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| spec/current-gap-matrix.md | source-analysis | local matrix rows GAP-* | none | app-cisdigital-ai-intent-forge original design and current tron skill files | AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-15, AC-MDPQ-17 |
| spec/requirements-flow-matrix.md | source-analysis | local matrix rows FLOW-001..FLOW-055 | none | memory-dreaming-workflow-design.md | AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13 |
| spec/requirements-governance-matrix.md | source-analysis | local matrix rows GOV-001..GOV-056 | none | memory-dreaming-workflow-design.md, memory-dreaming-task-prompt-contract-design.md, tech-design-v2-detail.md | AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-12, AC-MDPQ-17 |
| spec/requirements-long-term-memory-matrix.md | source-analysis | local matrix rows LTM-001..LTM-078 | none | intentforge-long-term-memory skill references, user supplemental requirement U1 | AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16 |
| spec/requirements-quality-matrix.md | source-analysis | local matrix rows QUALITY-001..QUALITY-085 | none | memory-dreaming task prompt contract and long-term memory evaluation references | AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| spec/test-plan.md | proof-plan | CMD-CODEX-CLI-SKILL-EVAL, CMD-DOCS-SPEC, CMD-LIVE-SMOKE-DRY, CMD-SKILL-MEMORY-DREAMING-QUALITY, CMD-SKILL-MEMORY-DREAMING-TEST, CMD-SKILL-VALIDATE | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06, AC-MDPQ-07, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| tasks/backend-task.md | task-plan | execution rows | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-07 |
| tasks/governance-task.md | task-plan | execution rows | none | none | AC-MDPQ-10 |
| tasks/private-evaluation-task.md | task-plan | execution rows | none | none | AC-MDPQ-11, AC-MDPQ-12 |
| tasks/quality-uplift-task.md | task-plan | execution rows | none | none | AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| tasks/review-task.md | task-plan | execution rows | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06, AC-MDPQ-07, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| tasks/skill-task.md | task-plan | execution rows | none | none | AC-MDPQ-08, AC-MDPQ-09 |
| tasks/test-task.md | task-plan | execution rows | none | none | AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06 |
| orchestration/feature-evolution-log.md | orchestration | supporting facts | none | none | AC-MDPQ-01, AC-MDPQ-04, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| orchestration/integration-log.md | orchestration | supporting facts | none | none | none |
| orchestration/worktree-plan.md | orchestration | supporting facts | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-07, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| reports/backend-result.md | evidence-ledger | evidence rows | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-07 |
| reports/private-evaluation-report.md | evidence-ledger | sanitized private evaluation report | Boundary, Method, Completion Evidence, Artifact Shape Evidence, Quality Findings, Governance Findings, MCP And Side-Effect Checks, Privacy Review, Acceptance Result, Residual Limits, Reviewer Conclusion | none | AC-MDPQ-11, AC-MDPQ-12 |
| reports/quality-uplift-report.md | evidence-ledger | evidence rows | Summary, Evidence Rows, Commands, Blockers | none | AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| reports/review-report.md | evidence-ledger | evidence rows | none | none | AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06, AC-MDPQ-07, AC-MDPQ-08, AC-MDPQ-09, AC-MDPQ-10, AC-MDPQ-11, AC-MDPQ-12, AC-MDPQ-13, AC-MDPQ-14, AC-MDPQ-15, AC-MDPQ-16, AC-MDPQ-17 |
| reports/test-report.md | evidence-ledger | evidence rows | none | none | AC-MDPQ-04, AC-MDPQ-05, AC-MDPQ-06 |
