import { useState } from "react";
import {
  Copy,
  Filter,
  MoreHorizontal,
  Plus,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SearchField,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useLocaleText,
  useTablePagination,
} from "@/shared";

import { usePromptList } from "@/domain/prompt/hooks";
import { promptCategoryFilters } from "@/domain/prompt/constants";
import type { PromptRecord } from "@/domain/prompt/model";
import { cn } from "@/shared/lib/utils";

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function PromptInspector({ prompt, notice, setNotice }: { prompt: PromptRecord; notice: string; setNotice: (value: string) => void }) {
  const pt = useLocaleText();
  const [variables, setVariables] = useState(prompt.variables);
  return (
    <SurfaceCard>
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-xl font-semibold">{pt("Prompt Details")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{prompt.name}</p>
        </div>
        <Button size="icon" variant="ghost" aria-label={pt("Close prompt details")} onClick={() => setNotice(pt("Prompt detail panel closed"))}><X size={16} aria-hidden="true" /></Button>
      </div>
      <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
        <pre className="whitespace-pre-wrap text-[13px] leading-6">{prompt.body}</pre>
      </div>
      <div className="mt-5 flex justify-end">
        <Button size="sm" variant="secondary" onClick={() => setNotice(pt("Prompt body copied"))}><Copy size={14} aria-hidden="true" />{pt("Copy")}</Button>
      </div>
      <div className="mt-6 grid gap-3">
        <h3 className="text-sm font-semibold">{pt("Test Variables")}</h3>
        {Object.entries(variables).map(([key, value]) => (
          <Field label={key} key={key}>
            {key === "user_query" ? (
              <TextArea value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            ) : (
              <TextInput value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            )}
          </Field>
        ))}
        <Button onClick={() => setNotice(pt("Preview ran for {{name}}", { name: prompt.name }))}>{pt("Run Test Preview")}</Button>
        <Notice value={notice} />
      </div>
    </SurfaceCard>
  );
}

export function PromptsPage() {
  const pt = useLocaleText();
  const prompts = usePromptList();
  const [category, setCategory] = useState("All");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(prompts[0]?.id ?? "");
  const [notice, setNotice] = useState("");
  const selected = prompts.find((prompt) => prompt.id === selectedId) ?? prompts[0];
  const filtered = prompts.filter((prompt) => (category === "All" || prompt.category === category) && prompt.name.toLowerCase().includes(query.toLowerCase()));
  const promptPagination = useTablePagination(filtered);

  return (
    <>
      <PageHeader
        title={pt("Prompt Management")}
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice(pt("{{count}} prompts exported", { count: filtered.length }))}>{pt("Bulk Export")}</Button>
            <Button onClick={() => setNotice(pt("New prompt flow opened"))}><Plus size={16} aria-hidden="true" />{pt("New")}</Button>
          </>
        }
      />
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <SurfaceCard className="p-0">
          <div className="border-b border-[var(--color-outline-variant)] px-5 pt-4">
            <div className="flex flex-wrap gap-1">
              {promptCategoryFilters.map((item) => (
                <button className={cn("border-b-2 px-4 py-2 text-sm", category === item ? "border-[var(--color-primary)] font-medium" : "border-transparent text-[var(--color-on-surface-variant)]")} type="button" key={item} onClick={() => setCategory(item)}>
                  {item === "All" ? pt("All Prompts") : pt(item)}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap items-center gap-2 py-4">
              <SearchField ariaLabel={pt("Search prompts")} placeholder={pt("Search prompts...")} value={query} onChange={setQuery} />
              <Button size="sm" variant="secondary" onClick={() => setNotice(pt("Prompt filters opened"))}><Filter size={14} aria-hidden="true" />{pt("Filter")}</Button>
            </div>
          </div>
          <DataTable
            variant="embedded"
            rows={promptPagination.visibleItems}
            rowKey={(row) => row.id}
            pagination={<TablePagination itemLabel={pt("prompts")} {...promptPagination.paginationProps} />}
            columns={[
              { id: "name", header: pt("Name"), render: (row) => <button className="font-medium text-[var(--color-primary)]" type="button" onClick={() => setSelectedId(row.id)}>{row.name}</button> },
              { id: "category", header: pt("Category"), render: (row) => <Badge>{pt(row.category)}</Badge> },
              { id: "inheritance", header: pt("Inheritance"), render: (row) => pt(row.inheritance) },
              { id: "updated", header: pt("Last Updated"), render: (row) => row.updated },
              { id: "actions", header: pt("Actions"), align: "right", render: (row) => <Button size="icon" variant="ghost" aria-label={pt("Select {{name}}", { name: row.name })} onClick={() => setSelectedId(row.id)}><MoreHorizontal size={16} aria-hidden="true" /></Button> },
            ]}
          />
        </SurfaceCard>
        <PromptInspector prompt={selected} notice={notice} setNotice={setNotice} />
      </div>
    </>
  );
}
