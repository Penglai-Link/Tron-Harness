import type { MineruNodeConfig } from "./SystemConfig";

export function validateMineruNodes(nodes: MineruNodeConfig[]): { key: string; node: string | number } | null {
  const ids = new Set<string>();
  for (const [index, node] of nodes.entries()) {
    const id = node.id.trim();
    if (!id || id === "local-markdown" || !/^[A-Za-z0-9_-]+$/.test(id)) return { key: "invalidId", node: index + 1 };
    if (ids.has(id)) return { key: "duplicateId", node: id };
    ids.add(id);
    if (![node.capacity, node.weight, node.resourceGroupCapacity].every((n) => Number.isSafeInteger(n) && n > 0) || node.weight > 4294967295) return { key: "invalidCapacity", node: id };
    let url: URL;
    try { url = new URL(node.baseUrl.trim()); } catch { return { key: "invalidUrl", node: id }; }
    if (!["http:", "https:"].includes(url.protocol) || !url.hostname || url.username || url.password || node.baseUrl.includes("?") || node.baseUrl.includes("#")) return { key: "invalidUrl", node: id };
    if (node.enabled && !["localhost", "127.0.0.1", "[::1]", "::1"].includes(url.hostname) && !node.allowRemote) return { key: "remoteConsent", node: id };
  }
  return null;
}
