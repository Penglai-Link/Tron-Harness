# Source Coverage Audit

status: source_audit_needs_matrix_thickening

This audit records where the current atomic matrices still compress the original design too much. Every added requirement must keep a source file and line range so implementation workers can compare the new skill behavior against the original design instead of relying only on summarized AC rows.

# Source Aliases

- `W` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/memory-dreaming-workflow-design.md`
- `P` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/memory-dreaming-task-prompt-contract-design.md`
- `T` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/intent-forge-control-plane/src/main/java/cn/cisdigital/ai/intent/forge/control/plane/memory/docs/tech-design-v2-detail.md`
- `L` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/.agents/skills/intentforge-long-term-memory/SKILL.md`
- `G` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/.agents/skills/intentforge-long-term-memory/references/memory-governance.md`
- `E` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/.agents/skills/intentforge-long-term-memory/references/memory-evaluation.md`
- `X` = `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge/.agents/skills/intentforge-long-term-memory/references/memory-examples.md`
- `U1` = user supplemental requirement, 2026-05-29 conversation: final testing and final quality rating must use local real Codex conversation data; current preparation records a testable protocol and does not execute that private run yet.

# Audit Summary

- Original reviewed source material is about 6090 lines; the first matrix pass had 238 requirement rows.
- The first pass preserved major directions but compressed field-level schemas, negative examples, dry-run governance semantics, authorization proof, and private evaluation rules.
- This audit adds matrix rows instead of replacing the original documents. Workers must use these rows as source pointers and reread the cited source sections when implementing or reviewing.
- User supplemental requirement `U1` upgrades the private real-history evaluation from an optional or later smoke check to a mandatory final quality gate.

# Thickening Backlog

| target_row | source | current coverage | missing or thin detail | required follow-up |
| --- | --- | --- | --- | --- |
| FLOW-045 | W:312-390; W:2141-2144 | FLOW-022 | `.intentforge-dreaming/` workspace has two views, `dream-run-id` format, and provider is not Memory scope or source adapter. | Add workspace/id/provider boundary row and directory/id tests. |
| FLOW-046 | W:314-367; W:406 | FLOW-008, FLOW-022 | Session artifacts are stable by `sourceHash`; no `sessions/<key>/<runId>/` analysis directory in V1. | Add stable session artifact row and second-run reuse fixture. |
| FLOW-047 | W:451-477; W:602-631 | FLOW-013, FLOW-014, FLOW-044 | Snapshot preprocessing needs normalized events, redaction markers, stable hashes, raw refs, and hints that cannot become facts. | Add snapshot preprocessing row and mutant fixtures. |
| FLOW-048 | W:721-794 | FLOW-016, FLOW-018 | Segment notes and session summary fields are not explicit enough, especially discarded findings and evidence index. | Add segment/session schema row and golden/mutant fixtures. |
| FLOW-049 | W:1190-1199; W:1980-1983 | FLOW-020, FLOW-024, FLOW-040 | Dry-run no-MCP mode must not be mistaken for a permanent ban on authorized Memory MCP execution in future modes. | Add dry-run vs executable governance row. |
| FLOW-050 | W:267-276; W:141-143 | FLOW-015, FLOW-039 | Source design forbids a global candidate pool; cross-session aggregation belongs to MemoryCard timeline. | Add no global candidate pool row. |
| FLOW-051 | W:2027-2041; W:1784-1792 | FLOW-036, FLOW-037 | DreamRun state graph needs explicit states and transition rules. | Add DreamRun state graph row. |
| FLOW-052 | W:1167-1170 | none | MCP write failure must preserve governance plan and retry evidence; it must not fake ACTIVE or swallow failure. | Add MCP write failure row. |
| FLOW-053 | W:249-256; W:1190-1218; W:2084; W:2110-2113 | FLOW-002 | Current-session instant save and ambiguous manual dreaming are outside the main batch DreamRun path. | Add entry boundary/current-session exclusion row. |
| FLOW-054 | W:958-965 | FLOW-004 | Time window is half-open and truncation by `endedAt` must be stable. | Add time-range/truncation detail row. |
| FLOW-055 | W:1328-1336; W:2025 | FLOW-029, FLOW-042 | No-value completion is represented by task output, not `task-skip`; oversized work is split by task type, not checkpoint. | Add command semantics row. |
| QUALITY-071 | P:126-168; P:169-189; P:1003-1045 | QUALITY-009, QUALITY-049, QUALITY-058, QUALITY-059 | Task prompt contract boundary, temp-output promotion, report ownership, and task-level MCP permission matrix are scattered. | Add task prompt contract boundary row. |
| QUALITY-072 | P:403-448 | none | `SEGMENT_SUMMARY` locality, event order, discarded findings, and forbidden full snapshot/cross-segment conclusion are missing. | Add segment local input and forbidden behavior row. |
| QUALITY-073 | P:403-448 | none | Segment notes quality checks need source separation, evidence locality, and summary usability. | Add segment notes quality row. |
| QUALITY-074 | P:449-490 | none | `SESSION_SUMMARY` finality, no material reread, and no MCP are not atomic. | Add session finality/compression row. |
| QUALITY-075 | P:449-490 | none | Discarded intermediate findings and evidence index must block Light pollution. | Add discarded findings/evidence index row. |
| QUALITY-076 | P:285-311 | QUALITY-022, QUALITY-023 | `sourceActor`, `sourceExpression`, and `ingestionChannel` affect confidence and lifecycle. | Add source-channel lifecycle row. |
| QUALITY-077 | P:366-389 | QUALITY-029, QUALITY-032, QUALITY-033 | Boundary score fixtures for 49/50, 54/55, 74/75 and stale/review threshold are missing. | Add threshold boundary row. |
| QUALITY-078 | P:770-815; P:831-841 | QUALITY-054, QUALITY-058 | REM match classes are compressed into enums; equality, supplement, conflict, rejected, and multi-card cases need separate proof. | Add REM match-class matrix row. |
| QUALITY-079 | P:857-903 | QUALITY-059, QUALITY-060 | Deep scan objects and no-op proof are too thin for candidate backlog, duplicates, stale cards, root bloat, and long cards. | Add Deep maintenance scan coverage row. |
| QUALITY-080 | P:894-903 | QUALITY-060 | Deep no-op must prove scope, budget, missing authorization, or no maintenance target. | Add Deep no-op proof row. |
| QUALITY-081 | P:905-917 | QUALITY-004, QUALITY-005, QUALITY-013 | Cross-task source attribution needs per-boundary prohibitions. | Add cross-task source attribution row. |
| QUALITY-082 | P:918-928 | QUALITY-007, QUALITY-037..QUALITY-040 | Memory type to default scope hint mapping is missing for PROCEDURE, RISK, and FACT. | Add type-to-scope-hint row. |
| QUALITY-083 | P:391-399 | scattered | Typical examples have not become fixture parity cases. | Add source examples fixture parity row. |
| QUALITY-084 | P:950-1001 | QUALITY-047, QUALITY-050, QUALITY-060 | Four core scenarios need full-chain golden/mutant fixtures. | Add core scenario full-flow fixture row. |
| QUALITY-085 | P:1081-1112 | GAP-003 | Prompt template metadata, taskType isolation, and prompt contract validation are missing. | Add versioned prompt contract metadata row. |
| GOV-049 | P:1017-1038; W:1190-1198 | GOV-030, GOV-031, GOV-032 | No-MCP dry-run must keep read/write intents but mark `executed=false`; it must not fake matched cards or timeline. | Add no-MCP dry-run boundary row. |
| GOV-050 | W:862-870; W:1055-1085; P:871-882 | GOV-018, GOV-030..GOV-032 | Proposed operation structure is not closed. | Add proposed operation schema row. |
| GOV-051 | P:871-882; P:1087-1089; W:1907; W:2102-2104 | GOV-021, GOV-022, GOV-025..GOV-027, GOV-048 | Authorization must be evidence, not only a boolean. | Add authorization evidence row. |
| GOV-052 | T:879; T:914-924; T:1005-1007; T:1052; T:1072 | GOV-036, GOV-039 | Revision conflict recovery is missing. | Add stale revision recovery row. |
| GOV-053 | T:240-249; T:472-485; T:1083-1112; T:1432-1441 | GOV-033, GOV-036 | MCP boundary must forbid GraphStore/Core direct writes and fake domain tools. | Add MCP bypass prohibition row. |
| GOV-054 | P:864; P:877; P:889-892; W:1897-1905; T:1105; T:1436 | GOV-023, GOV-024, GOV-027 | Merge/split needs save-only expression: primary card, reinforced card, disabled/archived duplicate, and timeline basis. | Add save-only merge/split row. |
| GOV-055 | P:768; P:815; P:868; P:881; T:938-954 | GOV-026 | Root and parent position changes need same-scope, real parent, no fake root id, no cycles, and parent count limits. | Add root/parent governance row. |
| GOV-056 | P:939-948; P:978-989; W:1903-1905; T:552-556; T:812-818 | GOV-024, GOV-025, GOV-039 | Conflict and stale categories need explicit timeline and `includeExpired/statuses` behavior. | Add conflict/stale classification row. |
| LTM-063 | X:5-17; E:41; E:89; E:183-184 | LTM-001, LTM-011, LTM-061 | Long-term memory is runtime context, not a fallback knowledge base; ordinary technical judgment still needs relevance checks. | Add runtime-context relevance row. |
| LTM-064 | L:24-26; E:70-75 | LTM-004, LTM-045, LTM-047 | Insufficient, conflicting, expired, or unclear memory requires boundary explanation, not confident answers. | Add boundary explanation row. |
| LTM-065 | L:88-91; X:17 | LTM-011..LTM-013 | Balance conservative relevance with no full-read and no ritual rereads. | Add relevance budget row. |
| LTM-066 | G:91-109 | LTM-013, LTM-029, LTM-062 | Tool choice table is compressed; timeline is not default ordinary read, save only after action and position are clear. | Add tool-choice matrix row. |
| LTM-067 | X:19-46; E:58-60; E:132; E:150 | LTM-024, LTM-029, LTM-044 | Write flow needs root browse, expansion, dedupe, main-card update, or leaf creation as one fixture. | Add write-flow fixture row. |
| LTM-068 | E:56; E:106; E:126; X:94 | none | Explicitly authorized sensitive/high-risk memory cannot be reduced to one-size rejection or reference-only storage. | Add authorized sensitive memory row. |
| LTM-069 | E:127; E:174; X:510-515 | LTM-017, LTM-021, LTM-049 | Upgrading personal preference to SPACE/team rule needs authority, scope, and time confirmation. | Add SPACE upgrade confirmation row. |
| LTM-070 | E:120; E:175; X:523-528 | LTM-023, LTM-025 | Session-summary facts need stable extraction, `sourceType=SESSION_SUMMARY`, and review points. | Add summary-pollution row. |
| LTM-071 | X:98-100; X:239-274; E:139; E:181 | LTM-031..LTM-035 | Root health cannot force all projects into one template tree. | Add project-specific root evolution row. |
| LTM-072 | G:223-233 | LTM-022, LTM-034, LTM-043 | Bad topic creation examples are missing. | Add topic naming anti-pattern row. |
| LTM-073 | X:276-351; X:451-458 | LTM-036..LTM-040 | Card quality needs `sourceDescription/sourceType`; Front Matter is for stable structured data, not every card. | Add card field completeness row. |
| LTM-074 | X:554-565; E:178; G:248-260 | LTM-027, LTM-047, LTM-060 | Expired/archived memory must not affect default usable memory; `includeExpired` is for governance/explanation only. | Add expired rule query row. |
| LTM-075 | X:530-535; E:129; E:146 | LTM-046 | Current-only override must not become long-term replacement unless user says it applies later. | Add current override vs long-term replacement row. |
| LTM-076 | G:248-260; X:546-572; E:135; E:176-177 | LTM-027, LTM-044 | Governance action mapping is too coarse for importance, error, history retention, no-longer-effective, and deletion. | Add governance action mapping row. |
| LTM-077 | E:173; E:180; E:182; X:421-428 | LTM-051..LTM-057 | Nickname not real identity, cross-domain mixed cards, and memory-impact explanations preserving scope are missing. | Add scope-subject extension fixture row. |
| LTM-078 | E:7-35; E:186-209 | LTM-061, LTM-062 | Private evaluation needs a record template, scenario counts, failure ids, and sanitized PASS/FAIL fields. | Add private evaluation record row. |
| AC-MDPQ-11/12 | U1; E:7-35; E:186-209 | AC-MDPQ-11, AC-MDPQ-12, LTM-078 | Final skill quality cannot be judged from synthetic fixtures, helper scripts, or dry-run planning alone; the final gate must use local real Codex history through a real Codex CLI Agent session. | Record mandatory real-data gate, execution timing, approval inputs, artifact isolation, and sanitized evidence rules in acceptance, test plan, and private evaluation task. |

# Release Rule

AC-MDPQ-08 through AC-MDPQ-12 must not be marked pass if any blocker/high row in this audit remains unmapped to a matrix row, task row, fixture plan, or explicit accepted out-of-scope decision. AC-MDPQ-11 and AC-MDPQ-12 also must not be marked pass unless the mandatory real-data gate from `U1` is run in the final validation stage and the committed evidence is sanitized.
