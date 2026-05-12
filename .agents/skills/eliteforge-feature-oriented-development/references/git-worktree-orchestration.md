# Git Worktree Orchestration

Use this reference before dispatching any subagent. This file is only an adapter between the feature workflow and the active EliteForge git skill; it is not a source of branch, worktree, main-agent, or subagent naming rules.

## Select The Git Skill

Before creating, renaming, moving, or merging any branch/worktree, read the project declaration and select exactly one git skill:

- `eliteforge-git-feature-oriented-spec`
- `eliteforge-git-feature-oriented-lite-spec`

If the project has not declared which git spec applies, stop before provisioning and ask the user to choose. Once selected, follow that git skill for:

- the main agent branch and main worktree path
- the multi-agent base branch and base worktree path
- the subagent branch and subagent worktree path
- `agentId`, subtask naming, path mirroring, merge direction, cleanup, commit, and MR/PR rules

Do not create or preserve feature-skill-defined names such as `eliteforge/integration/*` or `eliteforge/<unique-agent-id>/*`.

## Main Agent Branch And Worktree

The selected git skill decides whether the lead starts on a non multi-agent branch/worktree or a multi-agent `/base` branch/worktree. Once subagents are needed, the lead integrates through the selected git skill's multi-agent base branch. If an existing non multi-agent branch must enter multi-agent mode, use the selected git skill's conversion rule instead of keeping both branch shapes.

Record the resolved main branch, main worktree path, selected git skill, and base ref in `orchestration/worktree-plan.md`.

## Subagent Branch And Worktree

Each subagent receives exactly one resolved branch and one resolved worktree path generated from the selected git skill. A branch can be checked out by only one worktree at a time. When several workers have the same role, keep `agentId` and subtask naming exactly as the selected git skill requires; uniqueness belongs to the full subagent branch/worktree, not to an ad hoc feature-skill agent id.

## Creation Commands

Bundled scripts belong to the active skill directory, not the target repository root. Before running a helper script, set `skill_dir` to the absolute path of the current `eliteforge-feature-oriented-development` skill directory and call scripts through `"$skill_dir/scripts/..."`.

Create or switch to the selected-git-skill multi-agent base branch using the git skill's own branch template.

Create a worker worktree only after resolving the selected-git-skill branch and worktree path:

```bash
"$skill_dir/scripts/create_agent_worktree.sh" "<selected-git-skill-subagent-branch>" "<selected-git-skill-subagent-worktree-path>" "<base-ref>"
```

Manual equivalent:

```bash
git worktree add -b "<selected-git-skill-subagent-branch>" "<selected-git-skill-subagent-worktree-path>" "<base-ref>"
```

## Dispatch Rules

- Give every subagent an absolute worktree path resolved from the selected git skill.
- Give every subagent its selected-git-skill branch name and task IDs.
- Tell every subagent not to edit the main worktree.
- Tell every subagent to write only in its assigned write scope.
- Tell every subagent to write a report fragment in `docs/features/<feature-slug>/reports/agents/`.
- Do not dispatch GREEN workers until matching RED tests have been integrated into the selected-git-skill multi-agent base branch.
- Do not dispatch VERIFY workers until GREEN branches have been integrated.

## Integration Rules

The Main Agent should inspect each worker branch before integration:

```bash
git -C <worker-worktree> status --short
git -C <worker-worktree> diff --stat <base-ref>...HEAD
git -C <worker-worktree> diff --name-only <base-ref>...HEAD
```

Preferred integration is merge or cherry-pick from the worker branch into the selected-git-skill multi-agent base branch:

```bash
git switch "<selected-git-skill-multi-agent-base-branch>"
git merge --no-ff "<selected-git-skill-subagent-branch>"
```

Use patch apply only when branch merge is unavailable:

```bash
git -C <worker-worktree> diff --binary <base-ref> > <patch-file>
git apply --index <patch-file>
```

After integration, rerun the command required by the task row or the approved test scope.

## Conflict Rules

- The Main Agent resolves conflicts.
- Workers do not resolve conflicts caused by other workers unless a new task row explicitly assigns that conflict.
- If two workers changed the same file unexpectedly, stop parallel dispatch for that file family and record the conflict in `orchestration/integration-log.md`.
- Never discard a worker's changes without recording the reason.

## Cleanup Rules

Cleanup is optional until final acceptance. When cleanup is requested:

```bash
git worktree remove <worktree-path>
git branch -d "<selected-git-skill-subagent-branch>"
```

Do not delete branches or worktrees that still contain unintegrated changes.
