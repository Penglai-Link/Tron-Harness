import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTabVisibility } from "@/components/editable-tabs";
import { useTranslation } from "react-i18next";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";
import { cancelQualityRun, defaultQualityOptions, getQualityDataset, listQualityRuns, qualityRunActive, startQualityRun, type QualityMetrics } from "../api/knowledgeQualityApi";
import { KnowledgeQualityChat } from "./KnowledgeQualityChat";

export function KnowledgeQualityPanel({ libraryId, name, onClose, onPublished }: { libraryId: string; name: string; onClose: () => void; onPublished: () => Promise<unknown> }) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeQuality.${key}`);
  const user = useKnowledgeUser();
  const visible = useTabVisibility();
  const cache = useQueryClient();
  const [tab, setTab] = useState("results");
  const [options, setOptions] = useState(defaultQualityOptions);
  const requestId = useRef<string | null>(null);
  const key = ["knowledge-quality-runs", user, libraryId, i18n.resolvedLanguage];
  const jobs = useQuery({ queryKey: key, queryFn: ({ signal }) => listQualityRuns(libraryId, signal), enabled: visible, refetchInterval: (q) => visible && (qualityRunActive(q.state.data?.runs[0]) || tab === "chat") ? 2000 : false });
  const run = jobs.data?.runs[0];
  useEffect(() => { if (tab === "results" && visible) void cache.invalidateQueries({ queryKey: ["knowledge-quality-runs", user, libraryId] }); }, [tab, visible, cache, user, libraryId]);
  const dataset = useQuery({ queryKey: ["knowledge-quality-dataset", user, libraryId, run?.datasetRevision], queryFn: ({ signal }) => getQualityDataset(libraryId, signal), enabled: visible });
  const published = useRef(new Set<string>());
  const onPublishedRef = useRef(onPublished); onPublishedRef.current = onPublished;
  useEffect(() => { if (run?.published && !published.current.has(run.id)) { published.current.add(run.id); void onPublishedRef.current(); } }, [run?.id, run?.published]);
  const start = useMutation({ mutationFn: () => { requestId.current ??= Array.from(crypto.getRandomValues(new Uint8Array(16)), b => b.toString(16).padStart(2, "0")).join(""); return startQualityRun(libraryId, requestId.current, options); }, onSuccess: () => { requestId.current = null; void cache.invalidateQueries({ queryKey: key }); }, onError: () => { /* Keep request ID across ambiguous submission retries. */ } });
  const cancel = useMutation({ mutationFn: () => cancelQualityRun(libraryId, run!.id), onSuccess: () => cache.invalidateQueries({ queryKey: key }) });
  const format = (metrics: QualityMetrics | null | undefined, field: "recall" | "mrr" = "recall") => metrics ? new Intl.NumberFormat(i18n.resolvedLanguage, { style: "percent", maximumFractionDigits: 1 }).format(metrics[field]) : "—";
  const active = qualityRunActive(run);
  const valid = Number.isFinite(options.targetRecall) && options.targetRecall >= 0 && options.targetRecall <= 1 && options.maxTrials >= 1 && options.maxTrials <= 6 && Number.isInteger(options.maxTrials);
  return <Sheet open onOpenChange={open => { if (!open) onClose(); }}>
    <SheetContent className="data-[side=right]:w-full data-[side=right]:sm:max-w-3xl min-h-0 gap-0" aria-describedby="knowledge-quality-description">
      <SheetHeader className="pr-12"><SheetTitle>{text("title")} · {name}</SheetTitle><SheetDescription id="knowledge-quality-description">{text("description")}</SheetDescription></SheetHeader>
      <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col px-4 pb-4">
        <TabsList><TabsTrigger value="results">{text("results")}</TabsTrigger><TabsTrigger value="chat">{text("chat")}</TabsTrigger></TabsList>
        <TabsContent value="chat" className="min-h-0 flex-1 overflow-hidden"><KnowledgeQualityChat libraryId={libraryId} /></TabsContent>
        <TabsContent value="results" className="min-h-0 flex-1 overflow-auto space-y-5 pt-4">
          <div className="flex flex-wrap items-center gap-2"><Button disabled={active || start.isPending || !valid || jobs.isPending || Boolean(jobs.error)} onClick={() => start.mutate()}>{text(start.isPending ? "starting" : "start")}</Button>{active ? <Button variant="outline" disabled={cancel.isPending || run?.status === "CancelRequested"} onClick={() => cancel.mutate()}>{text("cancel")}</Button> : null}<Button variant="outline" disabled={jobs.isFetching} onClick={() => void jobs.refetch()}>{text("refresh")}</Button></div>
          <details><summary className="cursor-pointer text-sm text-muted-foreground">{text("advanced")}</summary><div className="mt-3 grid gap-3 sm:grid-cols-2"><label>{text("target")}<Input type="number" min={0} max={100} disabled={active || start.isPending} value={options.targetRecall * 100} onChange={e => { requestId.current = null; setOptions(o => ({ ...o, targetRecall: Number(e.target.value) / 100 })); }} /></label><label>{text("trials")}<Input type="number" min={1} max={6} disabled={active || start.isPending} value={options.maxTrials} onChange={e => { requestId.current = null; setOptions(o => ({ ...o, maxTrials: Number(e.target.value) })); }} /></label><p className="sm:col-span-2 text-sm text-muted-foreground">{text("budget")}</p></div></details>
          {[jobs.error, dataset.error, start.error, cancel.error].filter(Boolean).map((error, i) => <p key={i} role="alert" className="text-[var(--color-error)]">{error!.message}</p>)}
          {jobs.isPending ? <p role="status">{text("loading")}</p> : null}
          {!run && !jobs.isPending ? <p className="text-muted-foreground">{text("empty")}</p> : null}
          {run ? <>
            <p role="status">{t(`knowledgeQuality.statuses.${run.status}`, { defaultValue: run.status })}{active ? ` · ${t(`knowledgeQuality.stages.${run.stage}`, { defaultValue: run.stage.startsWith("Trial") ? t("knowledgeQuality.trialStage", { number: run.stage.slice(5) }) : run.stage })}` : ""}</p>
            <p className="text-sm">{run.published ? text("published") : run.targetReached ? text("alreadyGood") : !active && run.status === "Completed" ? text("noImprovement") : active ? text("working") : text("notPublished")}</p>
            {run.error ? <p role="alert" className="text-[var(--color-error)]">{run.error}</p> : null}
            <p className="text-sm text-muted-foreground">{(run.cases ?? dataset.data?.dataset?.cases)?.some(c => c.provenance === "Generated") ? text("generated") : dataset.data?.dataset ? text("human") : text("datasetPending")}</p>
            {run.published && !run.targetReached ? <p>{text("belowTarget")}</p> : null}
            <Table><TableHeader><TableRow><TableHead>{text("strategy")}</TableHead><TableHead>{text("training")}</TableHead><TableHead>{text("holdout")}</TableHead><TableHead>MRR</TableHead><TableHead>{text("chunks")}</TableHead></TableRow></TableHeader><TableBody>
              <TableRow><TableCell>{text("baseline")}</TableCell><TableCell>{format(run.baselineTraining)}</TableCell><TableCell>{format(run.baselineHoldout)}</TableCell><TableCell>{format(run.baselineHoldout, "mrr")}</TableCell><TableCell>—</TableCell></TableRow>
              {run.trials.map((trial, index) => <TableRow key={index} data-state={run.selectedTrial === index ? "selected" : undefined}><TableCell>{t(`knowledgeQuality.strategies.${trial.strategy.id}`, { defaultValue: trial.strategy.id })} · {trial.chunkConfig.maxChars}<span className="block text-xs text-muted-foreground">{trial.strategy.version}{run.selectedTrial === index ? ` · ${text("selected")}` : ""}</span>{trial.error ? <p role="alert">{trial.error}</p> : null}</TableCell><TableCell>{format(trial.training)}</TableCell><TableCell>{format(trial.holdout)}</TableCell><TableCell>{format(trial.holdout, "mrr")}</TableCell><TableCell>{trial.chunkCount}</TableCell></TableRow>)}
            </TableBody></Table>
            <p className="text-xs text-muted-foreground">{t("knowledgeQuality.usage", { calls: run.modelOperations, tokens: run.estimatedInputTokens })}</p>
            {run.missing.length ? <section><h3 className="font-medium">{text("missing")}</h3><ul className="mt-2 list-disc pl-5 space-y-1">{run.missing.map(m => <li key={m.caseId}>{(run.cases ?? dataset.data?.dataset?.cases)?.find(c => c.id === m.caseId)?.query ?? m.caseId}</li>)}</ul></section> : null}
            {run.warnings.length ? <details><summary>{text("notes")}</summary><ul className="mt-2 list-disc pl-5 text-sm">{run.warnings.map((w, i) => <li key={i}>{w}</li>)}</ul></details> : null}
          </> : null}
          {(jobs.data?.runs.length ?? 0) > 1 ? <details><summary>{text("history")}</summary><ul className="mt-2 space-y-2 text-sm">{jobs.data!.runs.slice(1).map(r => <li key={r.id}>{new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "short", timeStyle: "short" }).format(r.createdAt)} · {t(`knowledgeQuality.statuses.${r.status}`, { defaultValue: r.status })} · {format(r.baselineHoldout)} → {format(r.selectedTrial !== null ? r.trials[r.selectedTrial]?.holdout : r.baselineHoldout)}</li>)}</ul></details> : null}
        </TabsContent>
      </Tabs>
    </SheetContent>
  </Sheet>;
}
