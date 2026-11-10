import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { TablePagination } from "@/components/table-pagination";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useResourcePage } from "@/domain/resource/hooks";
import type { ResourceRecord } from "@/domain/resource/model";

export function TopicFileResourcePicker({ workspaceCode, pending, onSelect, onUpload }: {
  workspaceCode: string;
  pending: boolean;
  onSelect: (resource: ResourceRecord) => void;
  onUpload: (file: File) => Promise<void>;
}) {
  const { t } = useTranslation("shell");
  const [searchDraft, setSearchDraft] = useState("");
  const [search, setSearch] = useState("");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const query = useMemo(() => ({
    workspaceCode, workspaceName: "", context: "ALL" as const,
    kinds: ["FILE", "IMAGE", "PDF"] as const, search, current, size,
  }), [workspaceCode, search, current, size]);
  const { data, error, loading, refresh } = useResourcePage(query);
  const busy = pending || loading;

  function submitSearch() {
    if (busy) return;
    const next = searchDraft.trim();
    if (next === search && current === 1) refresh();
    else { setCurrent(1); setSearch(next); }
  }

  return <div className="space-y-4">
    <section aria-labelledby="topic-existing-resources" className="space-y-3">
      <h3 id="topic-existing-resources" className="text-sm font-medium">{t("topic.chooseResource")}</h3>
      <div className="flex gap-2">
        <Input aria-label={t("topic.search")} value={searchDraft} placeholder={t("topic.searchPlaceholder")} disabled={pending} onChange={(event) => setSearchDraft(event.target.value)} />
        <Button data-dialog-primary disabled={busy} onClick={submitSearch}>{t("topic.search")}</Button>
      </div>
      <div aria-busy={loading} className="max-h-[min(16rem,24dvh)] overflow-y-auto rounded border border-border p-1">
        {loading ? <p role="status" className="px-2 py-6 text-sm text-muted-foreground">{t("chatSidebar.loading")}</p>
          : error ? <div className="space-y-2 px-2 py-4"><p className="text-sm text-muted-foreground">{t("topic.resourceLoadFailed")}</p><Button variant="outline" disabled={pending} onClick={refresh}>{t("topic.retry")}</Button></div>
          : data.records.length ? data.records.map((resource) => <Button key={resource.resourceCode} variant="ghost" disabled={pending} className="h-auto w-full justify-start py-2 text-left" onClick={() => onSelect(resource)}>
            <span className="min-w-0">
              <span className="block truncate">{resource.name}</span>
              {resource.file?.fileName && resource.file.fileName !== resource.name ? <span className="block truncate text-xs text-muted-foreground">{resource.file.fileName}</span> : null}
              {resource.description ? <span className="block truncate text-xs text-muted-foreground">{resource.description}</span> : null}
            </span>
          </Button>) : <p role="status" className="px-2 py-6 text-sm text-muted-foreground">{t(search ? "topic.noMatchingResources" : "topic.noFileResources")}</p>}
      </div>
      {data.total > 0 ? <div data-dialog-enter="ignore" className="text-xs text-muted-foreground"><TablePagination
        startItem={(current - 1) * size + 1} endItem={Math.min(current * size, data.total)}
        totalItems={data.total} itemLabel={t("topic.fileResources")} page={current} pageCount={Math.max(1, Math.ceil(data.total / size))} pageSize={size}
        onPageChange={busy ? undefined : setCurrent}
        onPageSizeChange={busy ? undefined : (value) => { setSize(value); setCurrent(1); }}
      /></div> : null}
    </section>
    <Field className="border-t border-border pt-4">
      <FieldLabel htmlFor="topic-upload">{t("topic.upload")}</FieldLabel>
      <Input id="topic-upload" type="file" disabled={pending} onChange={(event) => {
        const file = event.target.files?.[0];
        event.currentTarget.value = "";
        if (file) void onUpload(file);
      }} />
    </Field>
  </div>;
}
