# Task: Agent Orchestrator Refactor

## Requirement
Implement phases 1-3 of the agent orchestration refactor: rename the current single-run orchestrator, extract reusable agent run context construction, and introduce a top-level AgentOrchestrator facade for future multi-agent orchestration.

## Acceptance Criteria
- [x] Current single-run `AgentOrchestrator` is renamed to `AgentRunExecutor` with matching result and lifecycle type names.
- [x] Single agent run context and runtime selection construction is reusable outside `AgentInvocationService`.
- [x] A top-level `AgentOrchestrator` exists and preserves current single-agent behavior.
- [ ] Backend checks and relevant tests pass through Makefile targets.

## Overall Status
- status: running
- process: 80%
- current_step: 4

## Steps
| step | description | status | note |
| --- | --- | --- | --- |
| 1 | Rename current single-run orchestrator to `AgentRunExecutor` | finished | commit: e84c5e9 |
| 2 | Extract reusable agent run context construction | finished | commit: 773eefd |
| 3 | Add top-level `AgentOrchestrator` facade | finished | commit: pending |
| 4 | Validate with Makefile backend checks and tests | running | commit: pending |

## Update Log
| time | status | process | update |
| --- | --- | --- | --- |
| 2026-06-27 11:25:53 +0800 | notrun | 0% | task initialized |
| 2026-06-27 11:25:53 +0800 | running | 5% | started implementation and verified clean git workspace |
| 2026-06-27 11:25:53 +0800 | running | 25% | renamed the single-run executor and verified with `make be-check` |
| 2026-06-27 11:25:53 +0800 | running | 55% | extracted `AgentRunContextBuilder` and verified with `make be-check` |
| 2026-06-27 11:25:53 +0800 | running | 80% | added top-level `AgentOrchestrator` facade and verified with `make be-check` |
