import { useCallback, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import { usePromptPage } from "@/domain/prompt/hooks/usePromptPage";
import { usePromptTemplateDetail } from "@/domain/prompt/hooks/usePromptTemplateDetail";
import { usePromptTranslation } from "@/domain/prompt/hooks/usePromptTranslation";
import type { PromptPageQuery } from "@/domain/prompt/model";
import { localizedPromptContent } from "@/domain/prompt/model/promptLocalization";

interface RolePromptTemplatePickerProps {
  value: string;
  disabled?: boolean;
  onApply: (promptBody: string) => void;
}

export function RolePromptTemplatePicker(props: RolePromptTemplatePickerProps) {
  const pt = usePromptTranslation();
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger render={<Button type="button" size="sm" variant="outline" disabled={props.disabled} />}>
        {pt("roleTemplates.open", "Role library")}
      </DialogTrigger>
      {open ? <RolePromptTemplatePickerContent {...props} onClose={() => setOpen(false)} /> : null}
    </Dialog>
  );
}

function RolePromptTemplatePickerContent({ value, disabled, onApply, onClose }: RolePromptTemplatePickerProps & { onClose: () => void }) {
  const pt = usePromptTranslation();
  const { currentLocale } = useLocaleFormatters();
  const [search, setSearch] = useState("");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [selectedCode, setSelectedCode] = useState("");
  const [replacement, setReplacement] = useState<string | null>(null);
  const handleSearchChange = useCallback((next: string) => {
    setSearch(next);
    setCurrent(1);
    setSelectedCode("");
  }, []);
  const query = useMemo<PromptPageQuery>(() => ({ current, size, search: search.trim() || undefined, category: "AGENT_ROLE", enabled: true }), [current, search, size]);
  const { page, loading, error, refresh } = usePromptPage(query);
  const detail = usePromptTemplateDetail(selectedCode);
  const selected = detail.record?.enabled && detail.record.category === "AGENT_ROLE" ? detail.record : null;
  const content = selected ? localizedPromptContent(selected, currentLocale) : null;

  function apply(body: string) {
    if (disabled) return;
    onApply(body);
    onClose();
  }

  return (
    <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-4xl" closeLabel={pt("actions.close", "Close")}>
      <DialogHeader>
        <DialogTitle>{pt("roleTemplates.title", "Choose a role template")}</DialogTitle>
        <DialogDescription className="sr-only">{pt("roleTemplates.description", "Preview a role template and copy it into the role prompt. You can edit it after applying.")}</DialogDescription>
      </DialogHeader>
      <SearchField ariaLabel={pt("roleTemplates.search", "Search role templates")} placeholder={pt("roleTemplates.searchPlaceholder", "Search names, codes, or descriptions...")} value={search} onChange={handleSearchChange} />
      <div className="grid min-h-64 gap-4 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
        <div className="max-h-80 space-y-1 overflow-y-auto" aria-busy={loading}>
          {loading ? <p role="status">{pt("roleTemplates.loading", "Loading templates...")}</p> : error ? <div role="alert"><p>{error.message}</p><Button type="button" variant="outline" onClick={refresh}>{pt("actions.refresh", "Refresh")}</Button></div> : page.records.length === 0 ? <p>{pt("roleTemplates.empty", "No role templates match your search.")}</p> : page.records.map((record) => {
            const item = localizedPromptContent(record, currentLocale);
            return <Button key={record.promptCode} type="button" variant={selectedCode === record.promptCode ? "secondary" : "ghost"} className="h-auto w-full min-w-0 justify-start whitespace-normal px-3 py-3 text-left" aria-pressed={selectedCode === record.promptCode} onClick={() => setSelectedCode(record.promptCode)}>
              <span className="min-w-0"><span className="block font-medium">{item.displayName}</span>{item.description ? <span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{item.description}</span> : null}</span>
            </Button>;
          })}
        </div>
        <section className="max-h-80 min-w-0 overflow-y-auto rounded-lg border p-3" aria-label={pt("roleTemplates.preview", "Template preview")} aria-busy={detail.loading}>
          {detail.loading ? <p role="status">{pt("roleTemplates.loading", "Loading templates...")}</p> : detail.error ? <div role="alert"><p>{detail.error.message}</p><Button type="button" variant="outline" onClick={detail.refresh}>{pt("actions.refresh", "Refresh")}</Button></div> : content ? <><h3 className="mb-3 font-medium">{content.displayName}</h3><p className="whitespace-pre-wrap break-words text-sm">{content.promptBody}</p></> : <p>{selectedCode ? pt("roleTemplates.unavailable", "This template is no longer available.") : pt("roleTemplates.select", "Select a template to preview its role prompt.")}</p>}
        </section>
      </div>
      <TablePagination page={current} pageCount={Math.max(1, Math.ceil(page.total / size))} pageSize={size} totalItems={page.total} startItem={page.records.length ? (current - 1) * size + 1 : 0} endItem={page.records.length ? (current - 1) * size + page.records.length : 0} itemLabel={pt("roleTemplates.itemLabel", "templates")} onPageChange={(next) => { setCurrent(Math.min(Math.max(1, next), Math.max(1, Math.ceil(page.total / size)))); setSelectedCode(""); }} onPageSizeChange={(next) => { setSize(next); setCurrent(1); setSelectedCode(""); }} />
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose}>{pt("actions.cancel", "Cancel")}</Button>
        <Button type="button" disabled={disabled || detail.loading || !content?.promptBody.trim()} onClick={() => { if (!content) return; if (value.trim()) setReplacement(content.promptBody); else apply(content.promptBody); }}>{pt("roleTemplates.apply", "Apply")}</Button>
      </DialogFooter>
      <Dialog open={replacement !== null} onOpenChange={(next) => { if (!next) setReplacement(null); }}>
        <DialogContent closeLabel={pt("actions.close", "Close")}>
          <DialogHeader>
            <DialogTitle>{pt("roleTemplates.confirmTitle", "Replace role prompt?")}</DialogTitle>
            <DialogDescription>{pt("roleTemplates.confirmDescription", "Applying this template will replace the entire existing role prompt.")}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setReplacement(null)}>{pt("actions.cancel", "Cancel")}</Button>
            <Button type="button" disabled={disabled} onClick={() => { if (replacement !== null) apply(replacement); }}>{pt("roleTemplates.confirmApply", "Replace and apply")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </DialogContent>
  );
}
