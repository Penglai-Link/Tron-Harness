import { MoreHorizontal } from "lucide-react";
import { useAuthStore } from "@/store/authStore";
import { Badge } from "@/components/ui/badge";
import { useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import { toastManager, useToast } from "@/components/ui/toast";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { Card, CardContent } from "@/components/ui/card";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { InfoTooltip } from "@/components/info-tooltip";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Table, TableHeader, TableHead, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { fetchPlatformModels } from "@/domain/model/api/modelApi";
import { fetchProviderCatalog } from "@/domain/model/api/modelProviderApi";
import { configureRagCorpus } from "../api/ragApi";
import type { RagCorpus, RagSettings } from "../model/RagModel";
import { useRagErrorToast } from "../hooks/useRagErrorToast";

export function sourcePath(corpus: RagCorpus) { return corpus.knowledgeBaseId ? `/settings/knowledge-base/${encodeURIComponent(corpus.knowledgeBaseId)}` : corpus.id === "skill" ? "/settings/skills" : "/settings/memory"; }
export function sourceActionKey(corpus: RagCorpus) { return corpus.knowledgeBaseId ? "ragUnified.manageKnowledgeBase" : corpus.id === "skill" ? "ragUnified.manageSkills" : "ragUnified.manageMemory"; }
export function RagDefinitionTab({ corpora, onSaved, onIngestion }: { corpora: RagCorpus[]; onSaved: () => void; onIngestion: (id: string) => void }) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const navigate = useNavigate();
  const dialogRef = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState<RagCorpus | null>(null);
  const [settings, setSettings] = useState<RagSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const models = useQuery({ queryKey: ["rag-platform-models", user], queryFn: ({ signal }) => fetchPlatformModels(signal), enabled: editing !== null });
  const providerCatalog = useQuery({ queryKey: ["rag-person-ai-catalog", user], queryFn: ({ signal }) => fetchProviderCatalog("PersonAi", signal), enabled: editing !== null });
  useRagErrorToast(editing ? models.error?.message : undefined);
  useRagErrorToast(editing ? providerCatalog.error?.message : undefined);
  const embeddingCatalog = new Map<string, { name: string; instances: number }>();
  for (const model of providerCatalog.data ?? []) {
    if (model.catalogModelType !== "Embedding" || model.status !== "Available" || !model.modelId) continue;
    const entry = embeddingCatalog.get(model.modelCode);
    if (entry) entry.instances += 1;
    else embeddingCatalog.set(model.modelCode, { name: model.modelName, instances: 1 });
  }
  const open = (corpus: RagCorpus) => { setEditing(corpus); setSettings({ ...corpus.settings, embeddingDimension: 1024 }); };
  const fieldLabel = (key: string, helpKey: string, id: string) => <div className="flex items-center gap-1.5">
    <FieldLabel htmlFor={id}>{t(`ragUnified.${key}`)}</FieldLabel>
    <InfoTooltip ariaLabel={t("ragUnified.fieldHelp", { field: t(`ragUnified.${key}`) })}>{t(`ragUnified.${helpKey}`)}</InfoTooltip>
  </div>;
  const save = async () => {
    if (!editing || !settings) return;
    setSaving(true);
    try {
      await configureRagCorpus(editing.id, settings);
      const corpus = editing;
      setEditing(null); onSaved();
      const toastId = toastManager.add({
        type: "success", title: t("ragUnified.configSaved"),
        description: corpus.counts.sourceCount > 0 ? t("ragUnified.savedRebuildHelp") : undefined,
        actionProps: corpus.counts.sourceCount > 0 ? {
          children: t("ragUnified.viewIngestion"),
          onClick: () => { toastManager.close(toastId); onIngestion(corpus.id); },
        } : undefined,
      });
    }
    catch (cause) { toast.error(cause instanceof Error ? cause.message : String(cause)); }
    finally { setSaving(false); }
  };
  return <>
    <Card><CardContent className="pt-6">
      <Table><TableHeader><TableRow>{["corpus", "source", "sources", "chunks", "vectors", "graph", "indexStatus", "actions"].map((key) => <TableHead key={key} className={key === "actions" ? "w-16 text-right" : undefined}>{t(`ragUnified.${key}`)}</TableHead>)}</TableRow></TableHeader>
        <TableBody>{corpora.map((corpus) => <TableRow key={corpus.id}>
          <TableCell className="font-medium">{corpus.kind === "builtin" ? t(`ragUnified.builtins.${corpus.id}`) : corpus.name}</TableCell>
          <TableCell>{t(`ragUnified.${corpus.knowledgeBaseId ? "knowledgeBase" : "builtin"}`)}</TableCell>
          <TableCell>{corpus.counts.sourceCount}</TableCell><TableCell>{corpus.counts.chunkCount}</TableCell><TableCell>{corpus.counts.vectorCount}</TableCell><TableCell>{corpus.counts.graphCount}</TableCell>
          <TableCell>{corpus.counts.rebuildSourceCount > 0 ?
            <Badge variant="warning" render={<button type="button" />} aria-label={t("ragUnified.viewRebuildTasks", { name: corpus.kind === "builtin" ? t(`ragUnified.builtins.${corpus.id}`) : corpus.name, count: corpus.counts.rebuildSourceCount })} onClick={() => onIngestion(corpus.id)}>
              {t("ragUnified.indexRebuildCount", { count: corpus.counts.rebuildSourceCount })}
            </Badge> : <Badge variant="outline">{t(`ragUnified.${corpus.counts.sourceCount === 0 ? "indexEmpty" : "indexReady"}`)}</Badge>}
          </TableCell>
          <TableCell className="text-right">
            <DropdownMenu>
              <DropdownMenuTrigger render={
                <Button variant="ghost" size="icon" aria-label={t("ragUnified.corpusActions", { name: corpus.kind === "builtin" ? t(`ragUnified.builtins.${corpus.id}`) : corpus.name })}>
                  <MoreHorizontal size={16} aria-hidden="true" />
                </Button>
              } />
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => open(corpus)}>{t("ragUnified.configure")}</DropdownMenuItem>
                <DropdownMenuItem disabled={corpus.counts.sourceCount === 0} onClick={() => onIngestion(corpus.id)}>{t("ragUnified.rebuild")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => navigate(sourcePath(corpus))}>{t(sourceActionKey(corpus))}</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </TableCell>
        </TableRow>)}{corpora.length === 0 ? <TableRow><TableCell colSpan={8}>{t("ragUnified.emptyCorpora")}</TableCell></TableRow> : null}</TableBody>
      </Table>
    </CardContent></Card>
    <Dialog open={editing !== null} onOpenChange={(open) => { if (!open && !saving) setEditing(null); }}>
      <DialogContent ref={dialogRef} initialFocus={() => dialogRef.current?.querySelector<HTMLElement>("#rag-fulltext") ?? null} closeLabel={t("ragUnified.close")}><DialogHeader>
        <div className="flex items-center gap-1.5 pr-8">
          <DialogTitle>{t("ragUnified.configure")} · {editing?.kind === "builtin" ? t(`ragUnified.builtins.${editing.id}`) : editing?.name}</DialogTitle>
          <InfoTooltip ariaLabel={t("ragUnified.fieldHelp", { field: t("ragUnified.configure") })}>{t("ragUnified.configHelp")}</InfoTooltip>
        </div>
        <DialogDescription className="sr-only">{t("ragUnified.configHelp")}</DialogDescription>
      </DialogHeader>
        {settings ? <div className="grid gap-4">
          {(["fulltext", "vector", "graph"] as const).map((key) => <Field key={key}>
            {fieldLabel(key, `${key}Help`, `rag-${key}`)}
            <SelectInput id={`rag-${key}`} aria-label={t(`ragUnified.${key}`)} value={settings[key] ? "enabled" : "disabled"} onValueChange={(value) => setSettings({ ...settings, [key]: value === "enabled" })}>
              <SelectItem value="enabled">{t("ragUnified.enabled")}</SelectItem>
              <SelectItem value="disabled">{t("ragUnified.disabled")}</SelectItem>
            </SelectInput>
          </Field>)}
          <Field>{fieldLabel("embeddingDimension", "dimensionHelp", "rag-dimension")}<Input id="rag-dimension" type="number" value={1024} disabled /></Field>
          {(["embeddingModelCode", "graphModelCode", "rerankModelCode"] as const).map((key) => <Field key={key}>{fieldLabel(key, `${key}Help`, `rag-${key}`)}<SelectInput id={`rag-${key}`} aria-label={t(`ragUnified.${key}`)} value={settings[key] ?? "__none__"} onValueChange={(value) => setSettings({ ...settings, [key]: value === "__none__" ? null : String(value) })}>
            <SelectItem value="__none__">{t(key === "rerankModelCode" ? "ragUnified.rerankOff" : "ragUnified.chooseModel")}</SelectItem>
            {key === "graphModelCode" ? <SelectItem value="PERSON_AI:ROUTE:ChatAuto">{t("ragUnified.autoRoute")}</SelectItem> : null}
            {key === "embeddingModelCode" ? [...embeddingCatalog].map(([code, model]) => <SelectItem key={`person-ai-${code}`} value={`PERSON_AI:EMBEDDING:${code}`}>{model.name} · Person-AI{model.instances > 1 ? ` (${t("ragUnified.instanceCount", { count: model.instances })})` : ""}</SelectItem>) : null}
            {(models.data ?? []).filter((model) => model.enabled && model.modelType === (key === "embeddingModelCode" ? "EMBEDDING" : key === "graphModelCode" ? "TEXT" : "RERANK")).map((model) => <SelectItem key={model.modelCode} value={model.modelCode}>{model.displayName}</SelectItem>)}
          </SelectInput>{key === "embeddingModelCode" && settings.vector && (!settings.embeddingModelCode || settings.embeddingModelCode === "PERSON_AI:ROUTE:EmbeddingAuto") ? <p className="text-sm text-destructive">{t("ragUnified.embeddingModelRequired")}</p> : null}</Field>)}
          {editing?.knowledgeBaseId ? <Field>
            {fieldLabel("imageDescriptionModelCode", "imageDescriptionModelCodeHelp", "rag-image-description")}
            <SelectInput id="rag-image-description" aria-label={t("ragUnified.imageDescriptionModelCode")} value={settings.imageDescriptionModelCode ?? "__none__"} onValueChange={(value) => setSettings({ ...settings, imageDescriptionModelCode: value === "__none__" ? null : String(value) })}>
              <SelectItem value="__none__">{t("ragUnified.imageSourceTextOnly")}</SelectItem>
              {(models.data ?? []).filter((model) => model.enabled && model.modelType === "TEXT" && (model.modelCapabilities.includes("VISION_UNDERSTANDING") || model.modelCapabilities.includes("MULTIMODAL_ALL"))).map((model) => <SelectItem key={model.modelCode} value={model.modelCode}>{model.displayName}</SelectItem>)}
            </SelectInput>
          </Field> : null}
        </div> : null}
        <DialogFooter><Button variant="outline" onClick={() => setEditing(null)} disabled={saving}>{t("ragUnified.close")}</Button><Button onClick={() => void save()} disabled={saving || !settings || (settings.vector && (!settings.embeddingModelCode || settings.embeddingModelCode === "PERSON_AI:ROUTE:EmbeddingAuto")) || (!!editing?.knowledgeBaseId && settings.graph && !settings.graphModelCode)}>{t("ragUnified.save")}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </>;
}
