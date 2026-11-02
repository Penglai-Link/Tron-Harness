/** Match Framework's largest-remainder allocation, including token rounding. */
export function allocateContextShares(shares: Record<string, number>, tokens: number): Record<string, number> {
  const entries = Object.entries(shares).map(([id, share]) => {
    const product = tokens * Math.max(0, share);
    return { id, tokens: Math.floor(product / 10_000), remainder: product % 10_000 };
  });
  const valid = Object.values(shares).reduce((sum, share) => sum + share, 0) === 10_000
    && Object.values(shares).every(share => Number.isInteger(share) && share >= 1 && share <= 10_000);
  if (valid) {
    const remaining = tokens - entries.reduce((sum, entry) => sum + entry.tokens, 0);
    [...entries].sort((a, b) => b.remainder - a.remainder || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
      .slice(0, remaining).forEach(entry => { entry.tokens += 1; });
  }
  return Object.fromEntries(entries.map(entry => [entry.id, entry.tokens]));
}
