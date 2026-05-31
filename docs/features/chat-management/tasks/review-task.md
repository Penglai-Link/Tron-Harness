# Review Task: Chat Management

## Execution Contract

- feature_slug: chat-management
- status: done
- execution_mode: solo

## TDD Contract

Review reused fresh backend, frontend, docs, and browser evidence.

## Task Rows

- AC-CM-01 | phase: review | owner: lead | scope: backend persistence/API evidence | command_ref: CMD-BE-TEST | status: done.
- AC-CM-02 | phase: review | owner: lead | scope: backend send orchestration evidence | command_ref: CMD-BE-TEST | status: done.
- AC-CM-03 | phase: review | owner: lead | scope: frontend workbench and browser evidence | command_ref: CMD-FE-CHECK | status: done.
- AC-CM-04 | phase: review | owner: lead | scope: right context and browser evidence | command_ref: CMD-FE-CHECK | status: done.
- AC-CM-05 | phase: review | owner: lead | scope: backend AG-UI stream contract and regression evidence | command_ref: CMD-BE-TEST | status: done.
- AC-CM-06 | phase: review | owner: lead | scope: frontend AG-UI event cards, streaming Markdown, and browser evidence | command_ref: CMD-FE-CHECK | status: done.
- AC-CM-07 | phase: review | owner: lead | scope: frontend send completion toast behavior and browser evidence | command_ref: CMD-FE-CHECK | status: done.
- AC-CM-08 | phase: review | owner: lead | scope: frontend agent selector and session `agentCode` persistence evidence | command_ref: CMD-FE-CHECK | status: done.
- AC-CM-09 | phase: review | owner: lead | scope: backend system agent runtime dispatch and CLI adapter evidence | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

Review ran in the selected Lite Spec base worktree.

## Integration Notes

Final verification included `rtk make be-test`, `rtk make fe-check`, docs validation, and Playwright browser smoke.

## Open Decisions

- None.
