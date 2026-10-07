import { useState } from "react";
import { useTranslation } from "react-i18next";
import { formatKnowledgeDocumentError } from "../model/knowledgeDocumentError";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import type { ChunkConfig, ChunkPreview, KnowledgeDocument, ParsingTier } from "../model/KnowledgeLibraryModel";
import { previewKnowledgeDocument, updateKnowledgeDocument, getParsingCapabilities } from "../api/knowledgeBaseApi";
import { formatDocumentLocation } from "../model/documentLocation";
import { ImageAnalysisStatus } from "./ImageAnalysisStatus";
import { useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function DocumentSettingsDialog({ workspace, libraryId, document, defaults, onClose, onChanged }: {
  workspace: string; libraryId: string; document: KnowledgeDocument; defaults: ChunkConfig;
  onClose: () => void; onChanged: () => Promise<unknown>;
}) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const active = useKnowledgeViewActive();
  const [inherit, setInherit] = useState(document.chunkConfig === null);
  const [config, setConfig] = useState(document.chunkConfig ?? defaults);
  const markdown = document.fileName.toLowerCase().endsWith(".md");
  const office = /\.(docx|xlsx)$/i.test(document.fileName);
  const [tier, setTier] = useState<ParsingTier | "inherit">(document.parsingTier ?? "inherit");
  const capabilities = useQuery({ queryKey: ["kb-parsing-capabilities", workspace], queryFn: ({ signal }) => getParsingCapabilities(workspace, signal), enabled: !markdown, retry: false });
  const [preview, setPreview] = useState<ChunkPreview | null>(null);
  const [persisted, setPersisted] = useState({ config: document.chunkConfig, tier: document.parsingTier ?? null });
  const nextConfig = inherit ? null : config;
  const nextTier = tier === "inherit" ? null : tier;
  const configEqual = nextConfig === null || persisted.config === null
    ? nextConfig === persisted.config
    : nextConfig.mode === persisted.config.mode && nextConfig.maxChars === persisted.config.maxChars
      && nextConfig.overlapChars === persisted.config.overlapChars && nextConfig.headingDepth === persisted.config.headingDepth;
  const dirty = !configEqual || (!markdown && nextTier !== persisted.tier);
  const valid = Number.isInteger(config.maxChars) && config.maxChars >= 200 && config.maxChars <= 8000
    && Number.isInteger(config.overlapChars) && config.overlapChars >= 0 && config.overlapChars <= config.maxChars / 4
    && Number.isInteger(config.headingDepth) && config.headingDepth >= 1 && config.headingDepth <= 6;
  const change = (next: Partial<ChunkConfig>) => { setConfig((value) => ({ ...value, ...next })); setPreview(null); };
  const save = useMutation({
    mutationFn: async (withPreview: boolean) => {
      const changed = dirty;
      if (changed) {
        const result = markdown ? await updateKnowledgeDocument(workspace, libraryId, document.id, nextConfig) : await updateKnowledgeDocument(workspace, libraryId, document.id, nextConfig, nextTier);
        if (result.error || result.status?.includes("FAILED")) { await onChanged(); throw new Error(result.error || text("indexFailed")); }
        if (!active.current) return null;
        setPersisted({ config: nextConfig, tier: nextTier });
        await onChanged();
        if (["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(result.status)) return { preview: null, changed, processing: true };
      }
      if (!active.current) return null;
      return { preview: withPreview ? await previewKnowledgeDocument(workspace, libraryId, document.id) : null, changed, processing: false };
    },
    onSuccess: (result) => { if (!active.current || !result) return; setPreview(result.preview); if (result.changed) toast.success(text(result.processing ? "processingStarted" : "saved")); },
    onError: (error) => { if (active.current) toast.error(formatKnowledgeDocumentError(error.message, t)); },
  });


  return <Dialog open onOpenChange={(open) => { if (!open && !save.isPending) onClose(); }}>
    <DialogContent closeLabel={text("close")} aria-describedby={undefined} className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{document.fileName}</DialogTitle></DialogHeader>
      {!markdown ? <Field label={text("parsingQuality")} hintAriaLabel={text("parsingQualityHelp")} hint={<div className="max-w-sm space-y-2 text-left">{["inherit", "flash", "basic", "standard", "advanced"].map((value) => <p key={value}>{text(`parsingTierHelp.${value}`)}</p>)}</div>}><SelectInput aria-label={text("parsingQuality")} value={tier} disabled={save.isPending || !capabilities.data} onValueChange={(value) => { setTier(value as ParsingTier | "inherit"); setPreview(null); }}>
        <SelectItem value="inherit">{t("knowledgeLibrary.inheritParsing", { tier: text(`parsingTiers.${office ? "flash" : capabilities.data?.defaultTier ?? "standard"}`) })}</SelectItem>
        {(capabilities.data?.availableTiers ?? []).filter((value) => !office || value === "flash").map((value) => <SelectItem key={value} value={value}>{text(`parsingTiers.${value}`)}</SelectItem>)}
      </SelectInput>
      {capabilities.error ? <p role="alert" className="text-[var(--color-error)]">{capabilities.error.message}</p> : null}
      {document.parsingArtifact ? <p className="text-xs">{t("knowledgeLibrary.actualParsing", { tier: text(`parsingTiers.${document.parsingArtifact.effectiveTier}`), version: document.parsingArtifact.parserVersion, count: document.parsingArtifact.imageResourceCodes.length })}</p> : null}
      </Field> : null}
      <label className="flex items-center justify-between gap-4"><span>{text("inherit")}</span><Switch checked={inherit} disabled={save.isPending} onCheckedChange={(value) => { setInherit(value); setPreview(null); }} /></label>
      <fieldset disabled={inherit || save.isPending} className="grid gap-4 disabled:opacity-60">
        <Field label={text("mode")}><SelectInput value={config.mode} onValueChange={(value) => change({ mode: value as ChunkConfig["mode"] })}>{(["AUTO", "HEADING", "FIXED"] as const).map((mode) => <SelectItem key={mode} value={mode}>{text(`modes.${mode}`)}</SelectItem>)}</SelectInput></Field>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={text("maxChars")}><TextInput type="number" min={200} max={8000} step={1} value={config.maxChars} onChange={(event) => change({ maxChars: Number(event.target.value) })} /></Field>
          <Field label={text("overlapChars")}><TextInput type="number" min={0} max={Math.floor(config.maxChars / 4)} step={1} value={config.overlapChars} onChange={(event) => change({ overlapChars: Number(event.target.value) })} /></Field>
        </div>
        {config.mode === "HEADING" ? <Field label={text("headingDepth")}><TextInput type="number" min={1} max={6} step={1} value={config.headingDepth} onChange={(event) => change({ headingDepth: Number(event.target.value) })} /></Field> : null}
      </fieldset>
      {inherit ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("knowledgeLibrary.defaults", { max: defaults.maxChars, overlap: defaults.overlapChars })}</p> : null}
      {!inherit && !valid ? <p role="alert" className="text-[var(--color-error)]">{text("invalidConfig")}</p> : null}
      {preview ? <section aria-label={text("preview")} className="grid gap-3">
        <p>{t("knowledgeLibrary.previewCount", { total: preview.chunkCount, shown: preview.chunks.length })}</p>
        {preview.warnings?.length ? <div className="text-sm text-[var(--color-warning)]"><p>{text("parseWarnings")}</p><ul className="mt-1 list-disc pl-4">{preview.warnings.filter((warning) => !preview.imageAnalysis || warning !== "图片原件和位置已保留；图片语义描述需要单独验证").map((warning, index) => <li key={index}>{warning === "图片原件和位置已保留；图片语义描述需要单独验证" ? text("imageNotice") : warning === "Markdown图片链接原样保留；仅已有资源引用可供授权Agent读取，不自动下载外部图片" ? text("markdownImageNotice") : warning}</li>)}</ul></div> : null}
        <ImageAnalysisStatus summary={preview.imageAnalysis} />
        {preview.chunks.map((chunk, index) => <div key={index} className="rounded-lg border border-[var(--color-card-border)] p-3"><p className="mb-2 text-xs text-[var(--color-on-surface-variant)]">{t("knowledgeLibrary.chunkNumber", { number: (chunk.ordinal ?? index) + 1 })} · {formatDocumentLocation(chunk.location, t, i18n.resolvedLanguage)}</p><pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words font-sans text-sm">{chunk.text}</pre></div>)}
      </section> : null}
      <DialogFooter>
        <Button variant="outline" disabled={save.isPending} onClick={onClose}>{text("close")}</Button>
        <Button variant="outline" disabled={save.isPending || (!inherit && !valid) || (!markdown && !capabilities.data)} onClick={() => save.mutate(true)}>{text(dirty ? "savePreview" : "previewAction")}</Button>
        <Button disabled={!dirty || save.isPending || (!inherit && !valid)} onClick={() => save.mutate(false)}>{text(save.isPending ? "saving" : "save")}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
