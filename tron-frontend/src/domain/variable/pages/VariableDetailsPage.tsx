import { useState } from "react";
import {
  ChevronRight,
  Eye,
  EyeOff,
  KeyRound,
  Plus,
  Search,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import { useVariableList } from "@/domain/variable/hooks";
import type { VariableRecord } from "@/domain/variable/model";


import { getStatusTone } from "@/domain/variable/constants";

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function VariablesBasePage({ variant }: { variant: "standard" | "details" }) {
  const variables = useVariableList();
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(variables[0]?.id ?? "");
  const [revealed, setRevealed] = useState(false);
  const [notice, setNotice] = useState("Variable inspector ready.");
  const selected = variables.find((variable) => variable.id === selectedId) ?? variables[0];
  const filtered = variables.filter((variable) => variable.key.toLowerCase().includes(query.toLowerCase()));

  return (
    <TronShell activeItem="Variables" searchPlaceholder="Search variables...">
      {variant === "details" ? <PageHeader title="Variable Management" status="Details" actions={<Button onClick={() => setNotice("New variable editor opened")}><Plus size={16} aria-hidden="true" />New Variable</Button>} /> : null}
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <SearchField placeholder="Search variables..." value={query} onChange={setQuery} />
            {variant === "standard" ? <Button onClick={() => setNotice("New variable editor opened")}><Plus size={16} aria-hidden="true" />New Variable</Button> : null}
          </div>
          <DataTable<VariableRecord>
            rows={filtered}
            rowKey={(row) => row.id}
            pagination={<span>Page 1 of 3 · {filtered.length} variables visible</span>}
            columns={[
              { id: "key", header: "Key", render: (row) => <button className="font-mono text-[13px] font-medium" type="button" onClick={() => setSelectedId(row.id)}>{row.key}</button> },
              { id: "scope", header: "Scope", render: (row) => <Badge>{row.scope}</Badge> },
              { id: "type", header: "Type", render: (row) => row.type },
              { id: "status", header: "Status", render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
              { id: "updated", header: "Updated", render: (row) => row.updated },
              { id: "actions", header: "", align: "right", render: (row) => <div className="flex justify-end gap-1"><Button size="icon" variant="ghost" aria-label={`Edit ${row.key}`} onClick={() => {
                setSelectedId(row.id);
                setNotice(`${row.key} selected for editing`);
              }}><KeyRound size={15} aria-hidden="true" /></Button><Button size="icon" variant="ghost" aria-label={`Delete ${row.key}`} onClick={() => {
                setSelectedId(row.id);
                setNotice(`Delete ${row.key} requested`);
              }}><X size={15} aria-hidden="true" /></Button></div> },
            ]}
          />
        </SurfaceCard>
        <SurfaceCard>
          <h2 className="text-xl font-semibold">{selected.key}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{selected.scope} · {selected.type}</p>
          <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
            <div className="flex items-center justify-between">
              <span className="font-mono text-sm">{revealed ? selected.valuePreview.replace("...", "_live_secret_") : selected.valuePreview}</span>
              <Button size="sm" variant="ghost" onClick={() => setRevealed((value) => !value)}>
                {revealed ? <EyeOff size={14} aria-hidden="true" /> : <Eye size={14} aria-hidden="true" />}
                {revealed ? "Hide" : "Reveal"}
              </Button>
            </div>
          </div>
          <h3 className="mt-8 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Usage in Prompts</h3>
          <div className="mt-3 space-y-2">
            {selected.usage.map((usage) => <button className="flex w-full items-center justify-between rounded-lg border border-[var(--color-outline-variant)] p-3 text-left text-sm hover:bg-[var(--color-surface-container-low)]" type="button" key={usage} onClick={() => setNotice(`${usage} usage link opened`)}><span>{usage}</span><ChevronRight size={15} aria-hidden="true" /></button>)}
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}

export function VariableDetailsPage() {
  return <VariablesBasePage variant="details" />;
}
