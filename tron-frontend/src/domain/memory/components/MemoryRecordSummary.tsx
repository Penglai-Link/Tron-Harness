import type { MemoryRecord } from "@/domain/memory/model";

export function MemoryRecordSummary({
  memory,
  onOpen,
}: {
  memory: MemoryRecord;
  onOpen: () => void;
}) {
  return (
    <div className="min-w-0">
      <button
        className="block w-full truncate text-left text-base font-medium leading-6 tracking-normal text-[#1a0dab] hover:underline"
        title={memory.title}
        type="button"
        onClick={onOpen}
      >
        {memory.title}
      </button>
      <p
        className="mt-1 line-clamp-2 whitespace-pre-wrap break-words text-[13px] leading-5 text-[var(--color-on-surface)] [overflow-wrap:anywhere]"
        data-testid="memory-record-text"
      >
        {memory.text}
      </p>
      <div
        className="mt-1 max-w-full truncate font-mono text-[12px] leading-5 text-[var(--color-on-surface-variant)]"
        title={memory.memoryCode}
      >
        {memory.memoryCode}
      </div>
    </div>
  );
}
