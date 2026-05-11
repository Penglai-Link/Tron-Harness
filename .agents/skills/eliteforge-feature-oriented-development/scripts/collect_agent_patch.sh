#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 3 ]]; then
  echo "Usage: $0 <worktree-path> <base-ref> <output-patch>" >&2
  exit 2
fi

worktree_path="$1"
base_ref="$2"
output_patch="$3"

if [[ ! -d "$worktree_path/.git" && ! -f "$worktree_path/.git" ]]; then
  echo "ERROR: not a git worktree: $worktree_path" >&2
  exit 2
fi

mkdir -p "$(dirname "$output_patch")"
git -C "$worktree_path" diff --binary "$base_ref" > "$output_patch"

echo "patch=$output_patch"
echo "changed_paths:"
git -C "$worktree_path" diff --name-only "$base_ref" | sed 's/^/- /'
