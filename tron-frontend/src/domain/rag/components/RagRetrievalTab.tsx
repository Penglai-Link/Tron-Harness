import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { ShieldCheck, Search, ListFilter, FileSearch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { InfoTooltip } from "@/components/info-tooltip";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Table, TableHeader, TableHead, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { searchRag } from "../api/ragApi";
import type { RagCorpus, RagSearchResult } from "../model/RagModel";
import { RagFlowSteps } from "./RagFlowSteps";

const retrievalSteps = [
  { stage: "authorize", icon: ShieldCheck },
  { stage: "recall", icon: Search },
  { stage: "fusion", icon: ListFilter },
  { stage: "evidence", icon: FileSearch },
];

export function RagRetrievalTab({ corpora, selected, onSelect }: { corpora: RagCorpus[]; selected: string; onSelect: (id: string) => void }) {
  const { t } = useTranslation("pages"); const navigate = useNavigate();
  const toast = useToast();
  const [query, setQuery] = useState(""); const [limit, setLimit] = useState(8);
  const [result, setResult] = useState<RagSearchResult | null>(null); const [pending, setPending] = useState(false);
  const clearResult = () => setResult(null);
  const search = async () => {
    setPending(true); clearResult();
    try {
      const response = await searchRag({ query, corpusIds: selected ? [selected] : [], limit });
      setResult(response);
      const failures = response.stages.filter((stage) => stage.error && !response.warnings.some((warning) => warning.includes(stage.error!)))
        .map((stage) => `${t(`ragUnified.channels.${stage.channel}`)}: ${stage.error}`);
      const notices = [...new Set([...response.warnings, ...failures])];
      if (notices.length) toast.warning(notices.join("\n"));
    }
    catch (cause) { toast.error(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(false); }
  };
  return <div className="grid gap-4">
    <RagFlowSteps label={t("ragUnified.retrievalFlow")} steps={retrievalSteps} />
    <Card><CardContent className="grid gap-4 pt-6">
      <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_10rem]">
        <Field><FieldLabel>{t("ragUnified.corpus")}</FieldLabel><SelectInput aria-label={t("ragUnified.corpus")} disabled={pending} value={selected || "__all__"} onValueChange={(value) => { onSelect(value === "__all__" ? "" : String(value)); clearResult(); }}><SelectItem value="__all__">{t("ragUnified.allAuthorized")}</SelectItem>{corpora.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}</SelectItem>)}</SelectInput></Field>
        <Field><FieldLabel htmlFor="rag-limit">{t("ragUnified.limit")}</FieldLabel><Input id="rag-limit" type="number" min={1} max={100} value={limit} onChange={(event) => setLimit(Number(event.target.value))} /></Field>
      </div>
      <Field><FieldLabel htmlFor="rag-question">{t("ragUnified.question")}</FieldLabel><Textarea id="rag-question" value={query} onChange={(event) => setQuery(event.target.value)} /></Field>
      <div className="flex justify-end"><Button disabled={pending || !query.trim() || !Number.isInteger(limit) || limit < 1 || limit > 100} onClick={() => void search()}>{t(pending ? "ragUnified.searching" : "ragUnified.search")}</Button></div>
    </CardContent></Card>
    {result ? <>
      <Card><CardContent className="grid gap-3 pt-6"><div className="flex flex-wrap items-center gap-2"><strong>{t("ragUnified.authorizedRange")}</strong>{result.authorizedCorpusIds.map((id) => <Badge key={id} variant="secondary">{corpora.find((item) => item.id === id)?.name ?? id}</Badge>)}<span>{result.durationMs} ms</span></div>
        <Table><TableHeader><TableRow>{["corpus", "channel", "state", "hits", "duration"].map((key) => <TableHead key={key}>{t(`ragUnified.${key}`)}</TableHead>)}</TableRow></TableHeader><TableBody>{result.stages.map((stage, i) => <TableRow key={i}><TableCell>{corpora.find((item) => item.id === stage.corpusId)?.name ?? stage.corpusId}</TableCell><TableCell>{t(`ragUnified.channels.${stage.channel}`)}</TableCell><TableCell><span className="inline-flex items-center gap-1.5">{t(`ragUnified.states.${stage.status}`)}{stage.error ? <InfoTooltip ariaLabel={t("ragUnified.stageFailureHelp", { channel: t(`ragUnified.channels.${stage.channel}`) })}>{stage.error}</InfoTooltip> : null}</span></TableCell><TableCell>{stage.hitCount}</TableCell><TableCell>{stage.durationMs} ms</TableCell></TableRow>)}</TableBody></Table>
      </CardContent></Card>
      {result.evidence.map((hit) => <Card key={`${hit.corpusId}:${hit.chunkCode}`}><CardContent className="grid gap-3 pt-6"><div className="flex flex-wrap items-center justify-between gap-3"><h3 className="font-medium">{hit.title ?? hit.sourceCode}</h3><span className="text-muted-foreground">{hit.score.toFixed(4)}</span></div><div className="flex gap-2">{hit.channels.map((channel) => <Badge key={channel} variant="secondary">{t(`ragUnified.channels.${channel}`)}</Badge>)}<Badge variant="outline">{hit.sourceVersion}</Badge></div><p className="whitespace-pre-wrap break-words">{hit.text}</p>{hit.location ? <dl className="flex flex-wrap gap-3 text-sm text-muted-foreground">{Object.entries(hit.location).filter(([, value]) => typeof value === "number" || typeof value === "string").map(([key, value]) => <div key={key}><dt className="inline">{t(`ragUnified.locations.${key}`, { defaultValue: key })}: </dt><dd className="inline">{String(value)}</dd></div>)}</dl> : null}{corpora.find((item) => item.id === hit.corpusId)?.knowledgeBaseId ? <Button variant="outline" className="justify-self-start" onClick={() => navigate(`/settings/knowledge-base/${encodeURIComponent(hit.corpusId)}?documentId=${encodeURIComponent(String(hit.source.documentId ?? ""))}`)}>{t("ragUnified.viewSource")}</Button> : null}</CardContent></Card>)}
    </> : null}
  </div>;
}
