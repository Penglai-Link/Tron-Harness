import { ChevronDown, ChevronRight, Folder, FolderOpen, Plus } from "lucide-react";
import { useId, useRef, useState, type KeyboardEvent } from "react";

import { Button } from "@/components/ui/button";
import type { WorkspaceTreeNodeRecord } from "../model";

interface WorkspaceTreeProps {
  records: WorkspaceTreeNodeRecord[];
  expandedCodes: Set<string>;
  selectedCode: string | null;
  label: string;
  archivedLabel: string;
  expandLabel: (name: string) => string;
  collapseLabel: (name: string) => string;
  createChildLabel: (name: string) => string;
  onToggle: (code: string) => void;
  onSelect: (code: string) => void;
  onCreateChild: (code: string) => void;
}

interface VisibleNode {
  record: WorkspaceTreeNodeRecord;
  depth: number;
  parentCode: string | null;
  position: number;
  siblings: number;
}

function visibleNodes(records: WorkspaceTreeNodeRecord[], expanded: Set<string>, depth = 0, parentCode: string | null = null): VisibleNode[] {
  return records.flatMap((record, index) => [
    { record, depth, parentCode, position: index + 1, siblings: records.length },
    ...(expanded.has(record.workspaceCode) ? visibleNodes(record.children, expanded, depth + 1, record.workspaceCode) : []),
  ]);
}

export function WorkspaceTree({ records, expandedCodes, selectedCode, label, archivedLabel, expandLabel, collapseLabel, createChildLabel, onToggle, onSelect, onCreateChild }: WorkspaceTreeProps) {
  const id = useId();
  const elements = useRef(new Map<string, HTMLDivElement>());
  const [focusedCode, setFocusedCode] = useState<string | null>(null);
  const nodes = visibleNodes(records, expandedCodes);
  const focusCode = nodes.some(node => node.record.workspaceCode === focusedCode) ? focusedCode
    : nodes.some(node => node.record.workspaceCode === selectedCode) ? selectedCode : nodes[0]?.record.workspaceCode;

  function focus(code: string | undefined | null) {
    if (!code) return;
    setFocusedCode(code);
    elements.current.get(code)?.focus();
  }

  function handleKey(event: KeyboardEvent<HTMLDivElement>, node: VisibleNode, index: number) {
    if (event.target !== event.currentTarget) return;
    const { record, parentCode } = node;
    const code = record.workspaceCode;
    switch (event.key) {
      case "ArrowDown": focus(nodes[index + 1]?.record.workspaceCode); break;
      case "ArrowUp": focus(nodes[index - 1]?.record.workspaceCode); break;
      case "Home": focus(nodes[0]?.record.workspaceCode); break;
      case "End": focus(nodes.at(-1)?.record.workspaceCode); break;
      case "ArrowRight":
        if (record.children.length) {
          if (!expandedCodes.has(code)) onToggle(code);
          else focus(nodes[index + 1]?.record.workspaceCode);
        }
        break;
      case "ArrowLeft":
        if (record.children.length && expandedCodes.has(code)) onToggle(code);
        else focus(parentCode);
        break;
      case "Enter": case " ": onSelect(code); break;
      default: return;
    }
    event.preventDefault();
    event.stopPropagation();
  }

  return <div role="tree" aria-label={label} className="space-y-1">
    {nodes.map((node, index) => {
      const { record, depth } = node;
      const code = record.workspaceCode;
      const selected = selectedCode === code;
      const expanded = expandedCodes.has(code);
      const hasChildren = record.children.length > 0;
      const Icon = hasChildren && expanded ? FolderOpen : Folder;
      return <div key={code} role="treeitem" aria-label={record.name} aria-describedby={`${id}-${code}`}
        aria-level={depth + 1} aria-posinset={node.position} aria-setsize={node.siblings}
        aria-selected={selected} aria-expanded={hasChildren ? expanded : undefined}
        tabIndex={focusCode === code ? 0 : -1}
        ref={element => { if (element) elements.current.set(code, element); else elements.current.delete(code); }}
        onFocus={event => { if (event.target === event.currentTarget) setFocusedCode(code); }}
        onKeyDown={event => handleKey(event, node, index)}
        onClick={() => { focus(code); onSelect(code); }}
        className={`group flex min-h-10 min-w-0 cursor-pointer items-center gap-1 rounded-md py-1 pr-1 text-sm outline-none transition-colors focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--color-focus-ring)] ${selected ? "bg-[var(--color-menu-selected-surface)] text-[var(--color-on-surface)]" : "text-[var(--color-on-surface)] hover:bg-[var(--color-menu-hover-surface)]"}`}
        style={{ paddingLeft: depth * 20 + 4 }}>
        {hasChildren ? <Button type="button" variant="ghost" size="icon-xs" tabIndex={-1} className="shrink-0 text-[var(--color-on-surface-variant)]"
          aria-label={expanded ? collapseLabel(record.name) : expandLabel(record.name)}
          onClick={event => { event.stopPropagation(); focus(code); onToggle(code); }}>
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </Button> : <span aria-hidden="true" className="size-6 shrink-0" />}
        <Icon size={16} aria-hidden="true" className="ml-0.5 shrink-0 text-[var(--color-on-surface-variant)]" />
        <span className={`min-w-0 flex-1 truncate px-1.5 ${selected ? "font-medium" : "font-normal"}`} title={`${record.name}\n${code}`}>{record.name}</span>
        <span id={`${id}-${code}`} className="sr-only">{code}</span>
        {record.status === "Archived" ? <span className="shrink-0 text-[10px] text-[var(--color-on-surface-variant)]">{archivedLabel}</span> : null}
        <Button type="button" variant="ghost" size="icon-xs" tabIndex={focusCode === code ? 0 : -1}
          className="shrink-0 text-[var(--color-on-surface-variant)] opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 [@media(hover:none)]:opacity-100"
          aria-label={createChildLabel(record.name)}
          onClick={event => { event.stopPropagation(); onCreateChild(code); }}><Plus size={14} /></Button>
      </div>;
    })}
  </div>;
}
