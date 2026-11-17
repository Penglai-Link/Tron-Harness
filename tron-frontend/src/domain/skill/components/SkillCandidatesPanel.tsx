import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTabVisibility } from "@/components/editable-tabs";
import { CodeEditor } from "@/components/files/CodeEditor";
import { SearchField } from "@/components/search-field";
import { SurfaceCard } from "@/shared";
import { actOnSkillCandidate, fetchSkillCandidate, fetchSkillCandidates, type SkillCandidateAction } from "../api/skillCandidateApi";
import { notifyCapabilityResourcesChanged } from "../api/skillApi";
import { canApproveSkillCandidate, compareSkillCandidateFiles, type SkillCandidateDetail } from "../model/SkillCandidate";

const candidateKeys = ["skill-candidates"] as const;
const activeStatuses = new Set(["Authoring", "WaitingDependency", "RevisionRequested", "Approved", "Installing"]);
const noEdit = () => undefined;
const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

export function SkillCandidatesPanel() {
  const { t } = useTranslation("pages");
  const text = (key: string, options?: Record<string, unknown>) => t(`skillCandidates.${key}`, options);
  const visibleTab = useTabVisibility();
  const [params, setParams] = useSearchParams();
  const candidateCode = params.get("candidateCode");
  const [search, setSearch] = useState("");
  const candidates = useQuery({
    queryKey: candidateKeys,
    enabled: visibleTab,
    queryFn: ({ signal }) => fetchSkillCandidates(signal),
    refetchInterval: (query) => visibleTab && query.state.data?.some((candidate) => activeStatuses.has(candidate.status)) ? 5000 : false,
  });
  const detail = useQuery({
    queryKey: [...candidateKeys, candidateCode],
    queryFn: ({ signal }) => fetchSkillCandidate(candidateCode!, signal),
    enabled: visibleTab && !!candidateCode,
    refetchInterval: (query) => visibleTab && query.state.data && activeStatuses.has(query.state.data.status) ? 3000 : false,
  });
  const select = (code: string | null) => setParams((previous) => {
    const next = new URLSearchParams(previous);
    if (code) next.set("candidateCode", code); else next.delete("candidateCode");
    next.set("tab", "approvals");
    return next;
  });
  if (candidateCode) return (
    <div className="space-y-4">
      <Button variant="outline" onClick={() => select(null)}>{text("back")}</Button>
      {detail.isPending ? <p role="status">{text("loading")}</p> : null}
      {detail.isError ? <div role="alert" className="space-y-2"><p>{errorMessage(detail.error)}</p><Button variant="outline" onClick={() => void detail.refetch()}>{text("retry")}</Button></div> : null}
      {detail.data ? <SkillCandidateReview key={candidateCode} candidate={detail.data} /> : null}
    </div>
  );
  const visible = candidates.data?.filter((candidate) => `${candidate.skillName} ${candidate.assistantCode}`.toLowerCase().includes(search.toLowerCase())) ?? [];
  return (
    <SurfaceCard className="p-0">
      <div className="flex flex-wrap gap-3 p-4">
        <SearchField className="min-w-0 flex-1" ariaLabel={text("search")} placeholder={text("search")} value={search} onChange={setSearch} />
        <Button variant="outline" disabled={candidates.isFetching} onClick={() => void candidates.refetch()}>{text("refresh")}</Button>
      </div>
      {candidates.isError ? <div role="alert" className="p-4"><p>{errorMessage(candidates.error)}</p><Button variant="outline" onClick={() => void candidates.refetch()}>{text("retry")}</Button></div> : null}
      {candidates.isPending ? <p role="status" className="p-4">{text("loading")}</p> : null}
      {!candidates.isPending && !candidates.isError && !visible.length ? <p className="p-4">{text("empty")}</p> : null}
      <ul className="divide-y divide-[var(--color-outline-variant)]">
        {visible.map((candidate) => (
          <li key={candidate.candidateCode}>
            <button type="button" className="flex w-full flex-wrap items-center gap-3 px-4 py-3 text-left hover:bg-[var(--color-surface-container)] focus-visible:outline focus-visible:outline-[var(--color-focus-ring)]" onClick={() => select(candidate.candidateCode)}>
              <span className="min-w-0 flex-1"><span className="block break-all font-medium">{candidate.skillName}</span><span className="text-xs text-[var(--color-on-surface-variant)]">{candidate.assistantCode} · {text(`operation.${candidate.operation}`)} · {text("revision", { revision: candidate.revision })}</span></span>
              <Badge variant={candidate.status === "Installed" ? "success" : candidate.status === "PendingApproval" ? "warning" : "outline"}>{text(`status.${candidate.status}`)}</Badge>
            </button>
          </li>
        ))}
      </ul>
    </SurfaceCard>
  );
}

function SkillCandidateReview({ candidate }: { candidate: SkillCandidateDetail }) {
  const { t } = useTranslation("pages");
  const text = (key: string, options?: Record<string, unknown>) => t(`skillCandidates.${key}`, options);
  const client = useQueryClient();
  const [explanation, setExplanation] = useState("");
  const files = compareSkillCandidateFiles(candidate);
  const [selectedPath, setSelectedPath] = useState("");
  const selectedFile = files.find((file) => file.path === selectedPath) ?? files[0];
  const mutation = useMutation({
    mutationFn: (action: SkillCandidateAction) => actOnSkillCandidate(candidate, action, explanation),
    onSuccess: async (result) => {
      client.setQueryData([...candidateKeys, candidate.candidateCode], result);
      setExplanation("");
      await client.invalidateQueries({ queryKey: candidateKeys });
      if (result.status === "Installed") notifyCapabilityResourcesChanged();
    },
    onError: () => { void client.invalidateQueries({ queryKey: candidateKeys }); },
  });
  const canReject = ["PendingApproval", "ValidationFailed", "RevisionRequested"].includes(candidate.status);
  const canRevise = ["PendingApproval", "ValidationFailed"].includes(candidate.status);
  const canRetry = ["InstallFailed", "Approved"].includes(candidate.status);
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div><h2 className="break-all text-lg font-medium">{candidate.skillName}</h2><p className="text-sm text-[var(--color-on-surface-variant)]">{candidate.assistantCode} · {text(`operation.${candidate.operation}`)} · {text("revision", { revision: candidate.revision })}</p></div>
        <Badge variant={candidate.status === "Installed" ? "success" : "outline"}>{text(`status.${candidate.status}`)}</Badge>
      </div>
      <dl className="grid gap-2 text-sm sm:grid-cols-[auto_minmax(0,1fr)]">
        <dt>{text("artifactHash")}</dt><dd className="break-all font-mono text-xs">{candidate.artifactHash}</dd>
        {candidate.baseArtifactHash ? <><dt>{text("baseHash")}</dt><dd className="break-all font-mono text-xs">{candidate.baseArtifactHash}</dd></> : null}
        <dt>{text("creator")}</dt><dd className="break-all">{candidate.creator ? `${candidate.creator.skillName} · ${candidate.creator.version} · ${candidate.creator.contentHash}` : text("missingCreator")}</dd>
      </dl>
      {candidate.lastError ? <p role="alert" className="text-[var(--color-error)]">{candidate.lastError}</p> : null}
      {candidate.reason ? <p>{text("rejectionReason")}: {candidate.reason}</p> : null}
      {candidate.feedback ? <p>{text("feedback")}: {candidate.feedback}</p> : null}
      <Tabs defaultValue="files">
        <TabsList variant="line">
          <TabsTrigger value="files">{text(candidate.operation === "Update" ? "filesAndDiff" : "files")}</TabsTrigger>
          <TabsTrigger value="evidence">{text("evidence")}</TabsTrigger>
          <TabsTrigger value="validation">{text("validation")}</TabsTrigger>
        </TabsList>
        <TabsContent value="files">
          <div className="flex flex-col gap-3 lg:flex-row">
            <div className="flex shrink-0 flex-wrap gap-1 lg:w-56 lg:flex-col" role="group" aria-label={text("files")}>
              {files.map((file) => <Button className="justify-start whitespace-normal break-all text-left" key={file.path} variant={selectedFile?.path === file.path ? "secondary" : "ghost"} aria-pressed={selectedFile?.path === file.path} onClick={() => setSelectedPath(file.path)}>{file.path}{candidate.operation === "Update" ? <Badge variant="outline">{text(`change.${file.change}`)}</Badge> : null}</Button>)}
            </div>
            {selectedFile ? <div className="min-w-0 flex-1 space-y-2">
              <p className="break-all font-mono text-xs">{selectedFile.path}</p>
              <div className={candidate.operation === "Update" ? "grid gap-3 xl:grid-cols-2" : ""}>
                {candidate.operation === "Update" ? <ArtifactFile label={text("installedVersion")} file={selectedFile.before} path={`installed/${selectedFile.path}`} empty={text("fileAbsent")} /> : null}
                <ArtifactFile label={text("proposedVersion")} file={selectedFile.after} path={`candidate/${candidate.artifactHash}/${selectedFile.path}`} empty={text("fileAbsent")} />
              </div>
            </div> : <p>{text("noFiles")}</p>}
          </div>
        </TabsContent>
        <TabsContent value="evidence">
          <ul className="divide-y divide-[var(--color-outline-variant)]">
            {candidate.evidence.map((evidence, index) => <li key={`${evidence.sourceCode}-${index}`} className="space-y-2 py-3"><p className="break-all text-xs font-mono">{evidence.workspaceCode} · {evidence.sourceCode} · {evidence.version}</p><p className="whitespace-pre-wrap break-words">{evidence.excerpt}</p></li>)}
          </ul>
          {!candidate.evidence.length ? <p>{text("noEvidence")}</p> : null}
        </TabsContent>
        <TabsContent value="validation">
          {candidate.validation ? <div className="space-y-3">
            <div className="flex flex-wrap gap-2"><Badge variant={candidate.validation.staticPassed ? "success" : "warning"}>{text("staticValidation")}: {text(candidate.validation.staticPassed ? "passed" : "failed")}</Badge><Badge variant={candidate.validation.behaviorPassed ? "success" : "warning"}>{text("behaviorValidation")}: {text(candidate.validation.behaviorPassed ? "passed" : "failed")}</Badge></div>
            {candidate.validation.artifactHash !== candidate.artifactHash ? <p role="alert">{text("staleValidation")}</p> : null}
            <ul className="divide-y divide-[var(--color-outline-variant)]">{candidate.validation.checks.map((check, index) => <li className="space-y-1 py-3" key={`${check.name}-${index}`}><p className="font-medium">{check.name} · {text(check.passed ? "passed" : "failed")}</p><p className="whitespace-pre-wrap break-words">{check.evidence}</p></li>)}</ul>
          </div> : <p>{text("noValidation")}</p>}
        </TabsContent>
      </Tabs>
      {mutation.isError ? <p role="alert" className="text-[var(--color-error)]">{errorMessage(mutation.error)}</p> : null}
      {canReject || canRevise ? <div className="space-y-2"><label htmlFor="skill-candidate-feedback" className="text-sm font-medium">{text("feedbackLabel")}</label><Textarea id="skill-candidate-feedback" value={explanation} onChange={(event) => setExplanation(event.target.value)} disabled={mutation.isPending} placeholder={text("feedbackPlaceholder")} /></div> : null}
      <div className="flex flex-wrap gap-2">
        {candidate.status === "PendingApproval" ? <Button disabled={mutation.isPending || !canApproveSkillCandidate(candidate)} onClick={() => mutation.mutate("approve")}>{text("approveInstall")}</Button> : null}
        {canRevise ? <Button variant="outline" disabled={mutation.isPending || !explanation.trim()} onClick={() => mutation.mutate("revise")}>{text("requestRevision")}</Button> : null}
        {canReject ? <Button variant="outline" disabled={mutation.isPending || !explanation.trim()} onClick={() => mutation.mutate("reject")}>{text("reject")}</Button> : null}
        {canRetry ? <Button disabled={mutation.isPending} onClick={() => mutation.mutate("retry-install")}>{text("retryInstall")}</Button> : null}
        {mutation.isPending ? <p role="status">{text("saving")}</p> : null}
      </div>
    </div>
  );
}

function ArtifactFile({ label, file, path, empty }: {
  label: string;
  file: SkillCandidateDetail["files"][number] | undefined;
  path: string;
  empty: string;
}) {
  return <section className="min-w-0 space-y-2" aria-label={label}><p className="text-xs text-[var(--color-on-surface-variant)]">{label}{file?.encoding === "Base64" ? " · Base64" : ""}</p>{file ? <div className="h-96 overflow-hidden rounded border border-[var(--color-outline-variant)]"><CodeEditor path={path} value={file.content} readOnly onChange={noEdit} onSave={noEdit} /></div> : <p className="text-sm">{empty}</p>}</section>;
}
