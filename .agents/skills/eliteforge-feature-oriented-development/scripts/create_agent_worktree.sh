#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <git-skill-subagent-branch> <git-skill-worktree-path> [base-ref]" >&2
  exit 2
fi

branch="$1"
worktree_path="$2"
base_ref="${3:-HEAD}"

if [[ ! "$branch" =~ ^[A-Za-z0-9._/-]+$ ]]; then
  echo "ERROR: branch must be a git-skill-compliant branch containing only letters, numbers, dots, underscores, hyphens, or slashes: $branch" >&2
  exit 2
fi

if [[ "$branch" == /* || "$branch" == -* || "$branch" == */ || "$branch" == */..* || "$branch" == *"//"* ]]; then
  echo "ERROR: branch must be a normalized relative branch path: $branch" >&2
  exit 2
fi

if [[ "$branch" == eliteforge/integration/* || "$branch" == eliteforge/* ]]; then
  echo "ERROR: branch uses the obsolete feature-skill naming scheme. Resolve the branch from eliteforge-git-feature-oriented-spec or lite-spec." >&2
  exit 2
fi

if [[ "$branch" != */*/* ]]; then
  echo "ERROR: branch is too short to be a selected-git-skill subagent branch: $branch" >&2
  exit 2
fi

if [[ "$worktree_path" != /* ]]; then
  echo "ERROR: worktree path must be absolute and resolved from the selected git skill: $worktree_path" >&2
  exit 2
fi

IFS='/' read -r -a branch_parts <<< "$branch"
IFS='/' read -r -a path_parts <<< "${worktree_path#/}"
if (( ${#path_parts[@]} < ${#branch_parts[@]} )); then
  echo "ERROR: worktree path must mirror the selected-git-skill branch tree: $worktree_path" >&2
  exit 2
fi
for ((i = 1; i <= ${#branch_parts[@]}; i++)); do
  branch_index=$((${#branch_parts[@]} - i))
  path_index=$((${#path_parts[@]} - i))
  if [[ "${path_parts[$path_index]}" != "${branch_parts[$branch_index]}" ]]; then
    echo "ERROR: worktree path must mirror the selected-git-skill branch tree: $worktree_path" >&2
    exit 2
  fi
done

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  echo "ERROR: not inside a git repository" >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel)"

case "$worktree_path" in
  "$repo_root"|"$repo_root"/*)
    echo "ERROR: worktree path must not be inside the main repository: $worktree_path" >&2
    exit 2
    ;;
esac

mkdir -p "$(dirname "$worktree_path")"

if git show-ref --verify --quiet "refs/heads/$branch"; then
  echo "ERROR: branch already exists: $branch" >&2
  echo "Use a different selected-git-skill subagent branch or remove the old branch after confirming it has no unmerged changes." >&2
  exit 1
fi

if [[ -e "$worktree_path" ]]; then
  echo "ERROR: worktree path already exists: $worktree_path" >&2
  exit 1
fi

git worktree add -b "$branch" "$worktree_path" "$base_ref"

cat <<OUT
branch=$branch
worktree_path=$worktree_path
base_ref=$base_ref
OUT
