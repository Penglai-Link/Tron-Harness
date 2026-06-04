# Backend Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: done
- execution_mode: solo

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY.

## Task Rows

- AC-TOOL-01 | phase: tdd | owner: lead | scope: extend code-backed tool catalog with `bash`/`fs`/`search` domains, concrete coding tools, and schema-level `description`/`examples` requirements plus backend API tests | command_ref: CMD-TOOL-BE-TEST | status: done.
- AC-TOOL-02 | phase: tdd | owner: lead | scope: preserve internal-agent policy persistence and effective assignment semantics while `all_platform_tools` now includes coding tools | command_ref: CMD-TOOL-BE-TEST | status: done.
- AC-TOOL-03 | phase: tdd | owner: lead | scope: register coding-tool executors, keep runtime invocation integration and domain-discovery materialization seam, and prove direct-list coding-tool execution through policy-derived tools | command_ref: CMD-TOOL-BE-TEST | status: done.

## Worktree Notes

- Lead branch: `feature/clouds3n/tool-registry/base`.
- This evolution runs solo in the current thread because no implementation subagent capability is available in this thread and the bounded coding-domain/backend scope would otherwise create same-file races with stale historical worker ownership.

## Integration Notes

- Catalog and policy should avoid ambiguous naming with existing runtime executor `ToolRegistry`.
- Main-agent invocation must derive effective tools when request tools are absent.
- `sed`, `awk`, `cat`, and similar Unix utilities are not first-class catalog tools in this iteration; they run through `bash.exec(command,args[])`.

## Open Decisions

- None.
