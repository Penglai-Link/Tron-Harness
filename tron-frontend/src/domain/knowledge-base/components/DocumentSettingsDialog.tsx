import { listChunkingStrategies } from "../api/knowledgeQualityApi";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatKnowledgeDocumentError } from "../model/knowledgeDocumentError";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import type { ChunkConfig, ChunkPreview, KnowledgeDocument, ParsingTier, OcrMode } from "../model/KnowledgeLibraryModel";
import { previewKnowledgeDocument, updateKnowledgeDocument, getParsingCapabilities } from "../api/knowledgeBaseApi";
import { formatDocumentLocation } from "../model/documentLocation";
import { currentImageWarnings, ImageAnalysisStatus } from "./ImageAnalysisStatus";
import { useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function DocumentSettingsDialog({ libraryId, document, defaults, onClose, onChanged }: {
  libraryId: string; document: KnowledgeDocument; defaults: ChunkConfig;
  onClose: () => void; onChanged: () => Promise<unknown>;
}) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const titleRef = useRef<HTMLHeadingElement>(null);
  const active = useKnowledgeViewActive();
  const user = useKnowledgeUser();
  const strategies = useQuery({ queryKey: ["chunking-strategies", user], queryFn: ({ signal }) => listChunkingStrategies(signal) });
  const builtins: Record<string, string> = { auto: "AUTO", structure: "Structure", heading: "HEADING", fixed: "FIXED" };
  const choices = strategies.data?.strategies.map(s => ({ ...s, mode: builtins[s.id] ?? s.id })) ?? [];
  const modeLabel = (id: string) => t(`knowledgeQuality.strategies.${id}`, { defaultValue: id });
  const [inherit, setInherit] = useState(document.chunkConfig === null);
  const [config, setConfig] = useState(document.chunkConfig ?? defaults);
  const markdown = /\.(md|markdown|txt|rst|tex)$/i.test(document.fileName);
  const legacy = /\.(doc|ppt|xls)$/i.test(document.fileName);
  const automatic = document.parsePlan?.preference === "Auto";
  const [ocrMode, setOcrMode] = useState<OcrMode | "inherit">(automatic ? "inherit" : document.ocrMode ?? "inherit");
  const [tier, setTier] = useState<ParsingTier | "inherit">(automatic ? "inherit" : document.parsingTier ?? "inherit");
  const parseMode = tier === "inherit" && ocrMode === "inherit" ? "inherit" : ocrMode === "Ocr" ? "Ocr" : tier === "advanced" ? "Precise" : "Text";
  const setParseMode = (mode: "inherit" | "Text" | "Ocr" | "Precise") => {
    setTier(mode === "inherit" ? "inherit" : mode === "Precise" ? "advanced" : "standard");
    setOcrMode(mode === "inherit" ? "inherit" : mode === "Ocr" ? "Ocr" : "Text");
    setPreview(null);
  };
  const capabilities = useQuery({ queryKey: ["kb-parsing-capabilities"], queryFn: ({ signal }) => getParsingCapabilities( signal), enabled: !markdown, retry: false });
  const [preview, setPreview] = useState<ChunkPreview | null>(null);
  const [persisted, setPersisted] = useState({ config: document.chunkConfig, tier: automatic ? null : document.parsingTier ?? null, ocrMode: automatic ? null : document.ocrMode ?? null });
  const nextConfig = inherit ? null : config;
  const nextOcrMode = ocrMode === "inherit" ? null : ocrMode;
  const nextTier = tier === "inherit" ? null : tier;
  const configEqual = nextConfig === null || persisted.config === null
    ? nextConfig === persisted.config
    : nextConfig.mode === persisted.config.mode && nextConfig.maxChars === persisted.config.maxChars
      && nextConfig.overlapChars === persisted.config.overlapChars && nextConfig.headingDepth === persisted.config.headingDepth;
  const dirty = !configEqual || (!markdown && nextTier !== persisted.tier) || (!markdown && nextOcrMode !== persisted.ocrMode);
  const valid = Number.isInteger(config.maxChars) && config.maxChars >= 200 && config.maxChars <= 8000
    && Number.isInteger(config.overlapChars) && config.overlapChars >= 0 && config.overlapChars <= config.maxChars / 4
    && Number.isInteger(config.headingDepth) && config.headingDepth >= 1 && config.headingDepth <= 6;
  const change = (next: Partial<ChunkConfig>) => { setConfig((value) => ({ ...value, ...next })); setPreview(null); };
  const save = useMutation({
    mutationFn: async (withPreview: boolean) => {
      const changed = dirty;
      if (changed) {
        const result = markdown ? await updateKnowledgeDocument( libraryId, document.id, nextConfig) : await updateKnowledgeDocument( libraryId, document.id, nextConfig, nextTier, nextOcrMode);
        if (result.error || result.status?.includes("FAILED")) { await onChanged(); throw new Error(result.error || text("indexFailed")); }
        if (!active.current) return null;
        setPersisted({ config: nextConfig, tier: nextTier, ocrMode: nextOcrMode });
        await onChanged();
        if (["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(result.status)) return { preview: null, changed, processing: true };
      }
      if (!active.current) return null;
      return { preview: withPreview ? await previewKnowledgeDocument( libraryId, document.id) : null, changed, processing: false };
    },
    onSuccess: (result) => { if (!active.current || !result) return; setPreview(result.preview); if (result.changed) toast.success(text(result.processing ? "processingStarted" : "saved")); },
    onError: (error) => { if (active.current) toast.error(formatKnowledgeDocumentError(error.message, t)); },
  });


  const parameters = <div className="grid gap-4 sm:grid-cols-2">
    <Field label={text("maxChars")} hintAriaLabel={text("maxCharsHelpLabel")} hint={text("maxCharsHelp")}><TextInput disabled={inherit || save.isPending} type="number" min={200} max={8000} step={1} value={config.maxChars} onChange={(event) => change({ maxChars: Number(event.target.value) })} /></Field>
    <Field label={text("overlapChars")} hintAriaLabel={text("overlapCharsHelpLabel")} hint={text("overlapCharsHelp")}><TextInput disabled={inherit || save.isPending} type="number" min={0} max={Math.floor(config.maxChars / 4)} step={1} value={config.overlapChars} onChange={(event) => change({ overlapChars: Number(event.target.value) })} /></Field>
  </div>;

  return <Dialog open onOpenChange={(open) => { if (!open && !save.isPending) onClose(); }}>
    <DialogContent initialFocus={titleRef} closeLabel={text("close")} aria-describedby={undefined} className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle ref={titleRef} tabIndex={-1} className="outline-none">{document.fileName}</DialogTitle></DialogHeader>
      {!markdown ? <Field label={text("parsingQuality")} hintAriaLabel={text("parsingQualityHelp")} hint={text("parseModeHelp")}>
        {legacy ? <p>{text("parseModes.Ocr")}</p> : <SelectInput aria-label={text("parsingQuality")} value={parseMode} disabled={save.isPending || !capabilities.data} onValueChange={(value) => setParseMode(value as "inherit" | "Text" | "Ocr" | "Precise")}>
          <SelectItem value="inherit">{text("parsingPreferences.Auto")}</SelectItem>
          {(["Text", "Ocr", "Precise"] as const).map((mode) => <SelectItem key={mode} value={mode}>{text(`parseModes.${mode}`)}</SelectItem>)}
        </SelectInput>}
      {capabilities.error ? <p role="alert" className="text-[var(--color-error)]">{capabilities.error.message}</p> : null}
      {document.parsingArtifact ? <p className="text-xs">{t("knowledgeLibrary.actualParsing", { tier: text(`parseModes.${document.ocrMode === "Ocr" ? "Ocr" : document.parsingArtifact.effectiveTier === "advanced" ? "Precise" : "Text"}`), version: document.parsingArtifact.parserVersion, count: document.parsingArtifact.imageResourceCodes.length })}</p> : null}
      </Field> : null}
      <label className="flex items-center justify-between gap-4"><span>{text("inherit")}</span><Switch checked={inherit} disabled={save.isPending} onCheckedChange={(value) => { setInherit(value); setPreview(null); }} /></label>
      <fieldset className="grid gap-4">
        {strategies.error ? <p role="alert">{strategies.error.message}</p> : null}
        <Field label={text("mode")} hintAriaLabel={text("modeHelpLabel")} hint={<div className="max-w-sm space-y-2 text-left">{["AUTO", "Structure", "HEADING", "FIXED"].map((mode) => <p key={mode}>{text(`modeHelp.${mode}`)}</p>)}</div>}><SelectInput disabled={inherit || save.isPending} aria-label={text("mode")} value={config.mode} onValueChange={(value) => change({ mode: value as ChunkConfig["mode"] })}>{choices.map(choice => <SelectItem key={choice.id} value={choice.mode}>{modeLabel(choice.id)}</SelectItem>)}{!choices.some(choice => choice.mode === config.mode) ? <SelectItem value={config.mode}>{t(`knowledgeLibrary.modes.${config.mode}`, { defaultValue: config.mode })}</SelectItem> : null}</SelectInput></Field>
        {config.mode === "AUTO" ? <details className="text-sm">
          <summary className="cursor-pointer text-[var(--color-on-surface-variant)]">{text("chunkParameters")}</summary>
          <div className="mt-4">{parameters}</div>
        </details> : parameters}
        {config.mode === "HEADING" ? <Field label={text("headingDepth")}><TextInput disabled={inherit || save.isPending} type="number" min={1} max={6} step={1} value={config.headingDepth} onChange={(event) => change({ headingDepth: Number(event.target.value) })} /></Field> : null}
      </fieldset>
      {inherit ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("knowledgeLibrary.defaults", { max: defaults.maxChars, overlap: defaults.overlapChars })}</p> : null}
      {!inherit && !valid ? <p role="alert" className="text-[var(--color-error)]">{text("invalidConfig")}</p> : null}
      {preview ? <section aria-label={text("preview")} className="grid gap-3">
        <p>{t("knowledgeLibrary.previewCount", { total: preview.chunkCount, shown: preview.chunks.length })}</p>
        {currentImageWarnings(preview.warnings ?? [], preview.imageAnalysis).length ? <div className="text-sm text-[var(--color-warning)]"><p>{text("parseWarnings")}</p><ul className="mt-1 list-disc pl-4">{currentImageWarnings(preview.warnings ?? [], preview.imageAnalysis).filter((warning) => !preview.imageAnalysis || warning !== "图片原件和位置已保留；图片语义描述需要单独验证").map((warning, index) => <li key={index}>{warning === "图片原件和位置已保留；图片语义描述需要单独验证" ? text("imageNotice") : warning === "Markdown图片链接原样保留；仅已有资源引用可供授权Agent读取，不自动下载外部图片" ? text("markdownImageNotice") : warning}</li>)}</ul></div> : null}
        <ImageAnalysisStatus summary={preview.imageAnalysis} />
        {preview.chunks.map((chunk, index) => <div key={index} className="rounded-lg border border-[var(--color-card-border)] p-3"><p className="mb-2 text-xs text-[var(--color-on-surface-variant)]">{t("knowledgeLibrary.chunkNumber", { number: (chunk.ordinal ?? index) + 1 })} · {formatDocumentLocation(chunk.location, t, i18n.resolvedLanguage)}</p><pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words font-sans text-sm">{chunk.text}</pre></div>)}
      </section> : null}
      <DialogFooter>
        <Button variant="outline" disabled={save.isPending} onClick={onClose}>{text("close")}</Button>
        <Button variant="outline" disabled={save.isPending || (!inherit && !valid) || (!markdown && !capabilities.data)} onClick={() => save.mutate(true)}>{text(dirty ? "savePreview" : "previewAction")}</Button>
        <Button disabled={!dirty || save.isPending || (!inherit && !valid) || (!markdown && !capabilities.data)} onClick={() => save.mutate(false)}>{text(save.isPending ? "saving" : "save")}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
