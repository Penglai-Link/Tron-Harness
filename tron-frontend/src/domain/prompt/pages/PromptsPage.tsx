import { useState } from "react";
import {
  Copy,
  Filter,
  MoreHorizontal,
  Plus,
  Search,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  TronShell,
  useTablePagination,
} from "@/shared";

import { usePromptList } from "@/domain/prompt/hooks";
import { promptCategoryFilters } from "@/domain/prompt/constants";
import type { PromptRecord } from "@/domain/prompt/model";
import { cn } from "@/shared/lib/utils";

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function PromptInspector({ prompt, notice, setNotice }: { prompt: PromptRecord; notice: string; setNotice: (value: string) => void }) {
  const [variables, setVariables] = useState(prompt.variables);
  return (
    <SurfaceCard>
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-xl font-semibold">Prompt Details</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{prompt.name}</p>
        </div>
        <Button size="icon" variant="ghost" aria-label="Close prompt details" onClick={() => setNotice("Prompt detail panel closed")}><X size={16} aria-hidden="true" /></Button>
      </div>
      <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
        <pre className="whitespace-pre-wrap text-[13px] leading-6">{prompt.body}</pre>
      </div>
      <div className="mt-5 flex justify-end">
        <Button size="sm" variant="secondary" onClick={() => setNotice("Prompt body copied")}><Copy size={14} aria-hidden="true" />Copy</Button>
      </div>
      <div className="mt-6 grid gap-3">
        <h3 className="text-sm font-semibold">Test Variables</h3>
        {Object.entries(variables).map(([key, value]) => (
          <Field label={key} key={key}>
            {key === "user_query" ? (
              <TextArea value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            ) : (
              <TextInput value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            )}
          </Field>
        ))}
        <Button onClick={() => setNotice(`Preview ran for ${prompt.name}`)}>Run Test Preview</Button>
        <Notice value={notice} />
      </div>
    </SurfaceCard>
  );
}

export function PromptsPage() {
  const prompts = usePromptList();
  const [category, setCategory] = useState("All");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(prompts[0]?.id ?? "");
  const [notice, setNotice] = useState("");
  const selected = prompts.find((prompt) => prompt.id === selectedId) ?? prompts[0];
  const filtered = prompts.filter((prompt) => (category === "All" || prompt.category === category) && prompt.name.toLowerCase().includes(query.toLowerCase()));
  const promptPagination = useTablePagination(filtered);

  return (
    <TronShell activeItem="Prompts" searchPlaceholder="Search anything...">
      <PageHeader
        title="Prompt Management"
        status="Split pane"
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice(`${filtered.length} prompts exported`)}>Bulk Export</Button>
            <Button onClick={() => setNotice("New prompt flow opened")}><Plus size={16} aria-hidden="true" />New Prompt</Button>
          </>
        }
      />
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <SurfaceCard className="p-0">
          <div className="border-b border-[var(--color-outline-variant)] px-5 pt-4">
            <div className="flex flex-wrap gap-1">
              {promptCategoryFilters.map((item) => (
                <button className={cn("border-b-2 px-4 py-2 text-sm", category === item ? "border-[var(--color-primary)] font-medium" : "border-transparent text-[var(--color-on-surface-variant)]")} type="button" key={item} onClick={() => setCategory(item)}>
                  {item === "All" ? "All Prompts" : item}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap items-center gap-2 py-4">
              <SearchField placeholder="Search prompts..." value={query} onChange={setQuery} />
              <Button size="sm" variant="secondary" onClick={() => setNotice("Prompt filters opened")}><Filter size={14} aria-hidden="true" />Filter</Button>
            </div>
          </div>
          <DataTable
            rows={promptPagination.visibleItems}
            rowKey={(row) => row.id}
            pagination={<TablePagination itemLabel="prompts" {...promptPagination.paginationProps} />}
            columns={[
              { id: "name", header: "Name", render: (row) => <button className="font-medium text-[var(--color-primary)]" type="button" onClick={() => setSelectedId(row.id)}>{row.name}</button> },
              { id: "category", header: "Category", render: (row) => <Badge>{row.category}</Badge> },
              { id: "inheritance", header: "Inheritance", render: (row) => row.inheritance },
              { id: "updated", header: "Last Updated", render: (row) => row.updated },
              { id: "actions", header: "Actions", align: "right", render: (row) => <Button size="icon" variant="ghost" aria-label={`Select ${row.name}`} onClick={() => setSelectedId(row.id)}><MoreHorizontal size={16} aria-hidden="true" /></Button> },
            ]}
          />
        </SurfaceCard>
        <PromptInspector prompt={selected} notice={notice} setNotice={setNotice} />
      </div>
    </TronShell>
  );
}
