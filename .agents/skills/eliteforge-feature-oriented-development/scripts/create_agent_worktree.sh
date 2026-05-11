#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <feature-slug> <agent-id> [base-ref]" >&2
  exit 2
fi

feature_slug="$1"
agent_id="$2"
base_ref="${3:-HEAD}"

if [[ ! "$feature_slug" =~ ^[a-z0-9][a-z0-9._-]*$ ]]; then
  echo "ERROR: feature-slug must be lowercase and contain only letters, numbers, dots, underscores, or hyphens: $feature_slug" >&2
  exit 2
fi

if [[ ! "$agent_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
  echo "ERROR: agent-id must contain only letters, numbers, dots, underscores, or hyphens: $agent_id" >&2
  exit 2
fi

if [[ "$agent_id" == "integration" ]]; then
  echo "ERROR: agent-id 'integration' is reserved for the lead-owned integration branch" >&2
  exit 2
fi

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  echo "ERROR: not inside a git repository" >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel)"
repo_name="$(basename "$repo_root")"
worktree_root="${ELITEFORGE_WORKTREE_ROOT:-$(dirname "$repo_root")/.eliteforge-worktrees/$repo_name}"
worktree_path="$worktree_root/eliteforge/$agent_id/$feature_slug"
branch="eliteforge/$agent_id/$feature_slug"

mkdir -p "$(dirname "$worktree_path")"

if git show-ref --verify --quiet "refs/heads/$branch"; then
  echo "ERROR: branch already exists: $branch" >&2
  echo "Use a new agent-id or remove the old branch after confirming it has no unmerged changes." >&2
  exit 1
fi

if [[ -e "$worktree_path" ]]; then
  echo "ERROR: worktree path already exists: $worktree_path" >&2
  exit 1
fi

git worktree add -b "$branch" "$worktree_path" "$base_ref"

cat <<OUT
agent_id=$agent_id
feature_slug=$feature_slug
branch=$branch
worktree_path=$worktree_path
base_ref=$base_ref
OUT
