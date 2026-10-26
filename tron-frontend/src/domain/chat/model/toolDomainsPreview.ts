type Domain = Record<string, unknown>;
const isDomain = (value: unknown): value is Domain => value !== null && typeof value === "object"
  && !Array.isArray(value) && typeof (value as Domain).domainCode === "string";

/** Decode only the complete string characters available in a truncated host record. */
function partialContextText(text: string): string | null {
  if (!/^\s*\{\s*"tronMessagePart"\s*:/.test(text) || !/"kind"\s*:\s*"tool_domains"/.test(text)) return null;
  const match = /"text"\s*:\s*"/.exec(text);
  if (!match) return null;
  const source = text.slice(match.index + match[0].length);
  let encoded = "";
  for (let index = 0; index < source.length;) {
    if (source[index] === '"') break;
    const size = source[index] === "\\" ? source[index + 1] === "u" ? 6 : 2 : 1;
    if (index + size > source.length) break;
    encoded += source.slice(index, index + size);
    index += size;
  }
  try { return JSON.parse(`"${encoded}"`) as string; } catch { return null; }
}

/** A saved request may end mid-array. Show complete records only, never invent missing fields. */
export function toolDomainsPreview(value: unknown): { domains: Domain[]; truncated: boolean } | null {
  if (typeof value !== "string") return null;
  let text = value.replace(/^\[Runtime reference data: tool_domains\]\s*/, "").trim();
  try {
    const parsed: unknown = JSON.parse(text);
    if (Array.isArray(parsed) && parsed.every(isDomain)) return { domains: parsed, truncated: false };
  } catch { /* The historical request can have been bounded mid-record. */ }
  text = partialContextText(text)?.trim() ?? text;
  if (!text.startsWith("[")) return null;
  const domains: Domain[] = [];
  let cursor = 1;
  while (cursor < text.length) {
    while (/\s/.test(text[cursor] ?? "") && cursor < text.length) cursor += 1;
    if (cursor === text.length) break;
    if (text[cursor] !== "{") return null;
    const start = cursor;
    const stack: string[] = [];
    let quoted = false;
    let escaped = false;
    for (; cursor < text.length; cursor += 1) {
      const char = text[cursor];
      if (quoted) {
        if (escaped) escaped = false;
        else if (char === "\\") escaped = true;
        else if (char === '"') quoted = false;
      } else if (char === '"') quoted = true;
      else if (char === "{" || char === "[") stack.push(char === "{" ? "}" : "]");
      else if (char === "}" || char === "]") {
        if (stack.pop() !== char) return null;
        if (stack.length === 0) { cursor += 1; break; }
      }
    }
    if (stack.length || quoted) break;
    try {
      const domain: unknown = JSON.parse(text.slice(start, cursor));
      if (!isDomain(domain)) return null;
      domains.push(domain);
    } catch { return null; }
    while (/\s/.test(text[cursor] ?? "") && cursor < text.length) cursor += 1;
    if (cursor === text.length) break;
    if (text[cursor] === "]") return { domains, truncated: true };
    if (text[cursor] !== ",") return null;
    cursor += 1;
  }
  return { domains, truncated: true };
}
