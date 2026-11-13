import type { ContextAssemblyBlock } from "./contextAssemblyReport";

/** Attribute only history usage above its configured share, not its unused capacity. */
export function contextBudgetBorrowing(blocks: readonly ContextAssemblyBlock[]) {
  const history = blocks.find((block) => block.id === "framework.history");
  const borrowedTokens = history?.configuredTokens != null
    ? Math.max(0, history.finalTokens - history.configuredTokens) : 0;
  const lentByBlock = new Map<string, number>();
  if (borrowedTokens === 0) return { borrowedTokens, lentByBlock, unallocatedTokens: 0 };

  const donors = blocks.flatMap((block) => {
    if (block.id === "framework.history" || block.configuredTokens === null) return [];
    const available = Math.max(0, block.configuredTokens - Math.max(block.finalTokens, block.allocatedTokens));
    return available > 0 ? [{ id: block.id, position: block.position, available: BigInt(available) }] : [];
  });
  const available = donors.reduce((total, donor) => total + donor.available, 0n);
  const lent = available < BigInt(borrowedTokens) ? available : BigInt(borrowedTokens);
  if (available > 0n) {
    // The report records a shared pool, not donor transfers. For display, spread
    // actual borrowing across unused block shares, then use unallocated capacity.
    // Largest remainders keep integer token totals exact and ordering stable.
    const shares = donors.map((donor) => ({
      ...donor,
      tokens: Number(lent * donor.available / available),
      remainder: lent * donor.available % available,
    }));
    let remaining = Number(lent) - shares.reduce((total, share) => total + share.tokens, 0);
    shares.sort((left, right) => left.remainder === right.remainder
      ? left.position - right.position || left.id.localeCompare(right.id)
      : left.remainder > right.remainder ? -1 : 1);
    for (const share of shares) {
      lentByBlock.set(share.id, share.tokens + (remaining > 0 ? 1 : 0));
      if (remaining > 0) remaining -= 1;
    }
  }
  return { borrowedTokens, lentByBlock, unallocatedTokens: borrowedTokens - Number(lent) };
}
