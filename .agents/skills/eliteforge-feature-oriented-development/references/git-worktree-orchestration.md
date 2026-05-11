# Git Worktree Orchestration

Use this reference before dispatching any subagent. The purpose is to isolate worker edits so subagents do not overwrite or revert each other.

## Branch Model

Recommended branch layout:

```text
<current-user-branch>
  └── eliteforge/integration/<feature-slug>
        ├── eliteforge/<agent-id-red-test>/<feature-slug>
        ├── eliteforge/<agent-id-green-backend>/<feature-slug>
        ├── eliteforge/<agent-id-green-frontend>/<feature-slug>
        ├── eliteforge/<agent-id-green-database>/<feature-slug>
        └── eliteforge/<agent-id-verify-review>/<feature-slug>
```

The integration branch is the only branch the Main Agent merges into during the feature workflow. Subagents work on task branches created from the appropriate integration-branch commit.

## Worktree Path Model

Recommended path outside the main repo:

```text
../.eliteforge-worktrees/<repo-name>/eliteforge/<agent-id>/<feature-slug>/
```

The `<agent-id>` segment must come before `<feature-slug>`. Do not place worktrees inside tracked application directories. If a `.gitignore` update is needed for a local worktree root, the Main Agent owns that change.

## Creation Commands

Bundled scripts belong to the active skill directory, not the target repository root. Before running a helper script, set `skill_dir` to the absolute path of the current `eliteforge-feature-oriented-development` skill directory and call scripts through `"$skill_dir/scripts/..."`.

Create the integration branch if it does not exist:

```bash
git switch -c eliteforge/integration/<feature-slug>
```

Create a worker worktree from a base ref:

```bash
"$skill_dir/scripts/create_agent_worktree.sh" <feature-slug> <agent-id> <base-ref>
```

Manual equivalent:

```bash
repo_root="$(git rev-parse --show-toplevel)"
repo_name="$(basename "$repo_root")"
worktree_root="${ELITEFORGE_WORKTREE_ROOT:-$(dirname "$repo_root")/.eliteforge-worktrees/$repo_name}"
worktree_path="$worktree_root/eliteforge/<agent-id>/<feature-slug>"
mkdir -p "$(dirname "$worktree_path")"
git worktree add -b "eliteforge/<agent-id>/<feature-slug>" "$worktree_path" "<base-ref>"
```

## Dispatch Rules

- Give every subagent an absolute worktree path.
- Give every subagent its branch name and task IDs.
- Tell every subagent not to edit the main worktree.
- Tell every subagent to write only in its assigned write scope.
- Tell every subagent to write a report fragment in `docs/features/<feature-slug>/reports/agents/`.
- Do not dispatch GREEN workers until matching RED tests have been integrated into the integration branch.
- Do not dispatch VERIFY workers until GREEN branches have been integrated.

## Integration Rules

The Main Agent should inspect each worker branch before integration:

```bash
git -C <worker-worktree> status --short
git -C <worker-worktree> diff --stat <base-ref>...HEAD
git -C <worker-worktree> diff --name-only <base-ref>...HEAD
```

Preferred integration is merge or cherry-pick from the worker branch into the integration branch:

```bash
git switch eliteforge/integration/<feature-slug>
git merge --no-ff eliteforge/<agent-id>/<feature-slug>
```

Use patch apply only when branch merge is unavailable:

```bash
git -C <worker-worktree> diff --binary <base-ref> > /tmp/<agent-id>.patch
git apply --index /tmp/<agent-id>.patch
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
git branch -d eliteforge/<agent-id>/<feature-slug>
```

Do not delete branches or worktrees that still contain unintegrated changes.
