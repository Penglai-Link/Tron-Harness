import type { MemoryRecord } from "../model";
export function MemoryRecordSummary({
  memory,
  onOpen,
}: {
  memory: MemoryRecord;
  onOpen: () => void;
}) {
  const characters = Array.from(memory.text);
  const preview = characters.slice(0, 220).join("") + (characters.length > 220 ? "…" : "");
  return (
    <button className="min-w-0 w-full text-left" onClick={onOpen}>
      <span className="block truncate font-medium">
        {memory.title || memory.memoryCode}
      </span>
      <span className="mt-1 line-clamp-2 whitespace-pre-wrap break-words text-sm text-[var(--color-on-surface-variant)]">
        {preview}
      </span>
    </button>
  );
}
