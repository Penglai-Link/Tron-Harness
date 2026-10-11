/** Preserve local edits while applying server changes and revoking unavailable resources. */
export function reconcileResourceDraft(
  draft: readonly string[],
  previous: readonly string[],
  incoming: readonly string[],
  available: readonly string[]
): string[] {
  const local = new Set(draft);
  const baseline = new Set(previous);
  const remote = new Set(incoming);
  const allowed = new Set(available);
  return [...new Set([...draft, ...incoming])].filter(
    (code) =>
      allowed.has(code) &&
      (local.has(code) !== baseline.has(code)
        ? local.has(code)
        : remote.has(code))
  );
}
