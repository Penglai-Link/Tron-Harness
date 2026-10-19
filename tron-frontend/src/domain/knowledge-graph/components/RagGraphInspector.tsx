import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { InspectorPanel } from "@/components/inspector";
import type { RagGraphNode, RagGraphEdge, RagGraphReference, RagGraphDirection } from "../model/RagGraphModel";

export interface RagGraphInspectorProps {
  ragNode: RagGraphNode; ragNodes: RagGraphNode[]; ragEdges: RagGraphEdge[]; ragReferences: RagGraphReference[];
  onClose: () => void; onExpand: (direction: RagGraphDirection) => void; onSelect: (id: string) => void;
  pending: boolean; incomingDepth: number; outgoingDepth: number;
}
export function RagGraphInspector({ ragNode: node, ragNodes: nodes, ragEdges: edges, ragReferences: references, onClose, onExpand, onSelect, pending, incomingDepth, outgoingDepth }: RagGraphInspectorProps) {
  const { t } = useTranslation("pages"); const navigate = useNavigate();
  const sourceIds = new Set([...node.referenceIds, ...edges.flatMap((edge) => edge.referenceIds)]);
  const sources = references.filter((reference) => sourceIds.has(reference.id));
  const names = new Map(nodes.map((item) => [item.id, item.name]));
  return <InspectorPanel title={t("knowledgeGraph.ragInspector")}>
    <div className="flex items-start justify-between gap-3"><div className="min-w-0"><h2 className="break-words text-lg font-semibold">{node.name}</h2><Badge variant="outline">{node.entityType ?? t(`knowledgeGraph.ragNodeKinds.${node.kind}`, { defaultValue: node.kind })}</Badge></div>
      <Button size="icon" variant="ghost" aria-label={t("knowledgeGraph.inspector.closeAria")} onClick={onClose}><X aria-hidden="true" size={16} /></Button></div>
    {node.aliases.length ? <p>{t("knowledgeGraph.ragAliases")}: {node.aliases.map((alias) => alias.text).join(", ")}</p> : null}
    {node.descriptions.map((description) => <p className="whitespace-pre-wrap break-words text-sm" key={`${description.referenceId}:${description.text}`}>{description.text}</p>)}
    {node.hops.length ? <section className="space-y-1"><h3 className="font-medium">{t("knowledgeGraph.ragHitReason")}</h3>{node.hops.map((hop) => <p className="text-sm text-muted-foreground" key={`${hop.seedId}:${hop.direction}`}>{t("knowledgeGraph.ragHop", { direction: t(`knowledgeGraph.ragDirections.${hop.direction}`), count: hop.hops })}</p>)}</section> : null}
    <div className="flex flex-wrap gap-2"><Button variant="outline" disabled={pending || incomingDepth === 0} onClick={() => onExpand("Incoming")}>{t("knowledgeGraph.ragExpandIncoming")}</Button><Button variant="outline" disabled={pending || outgoingDepth === 0} onClick={() => onExpand("Outgoing")}>{t("knowledgeGraph.ragExpandOutgoing")}</Button></div>
    <section className="space-y-2"><h3 className="font-medium">{t("knowledgeGraph.ragRelationships")}</h3>{edges.map((edge) => {
      const otherId = edge.from === node.id ? edge.to : edge.from;
      const otherName = names.get(otherId) ?? otherId;
      return <div className="border-b py-2 text-sm" key={edge.id}>
        <p className="break-words">{names.get(edge.from) ?? edge.from} → {edge.relationship} → {names.get(edge.to) ?? edge.to}</p>
        <Button variant="link" className="h-auto max-w-full whitespace-normal px-0 text-left" aria-label={`${t("knowledgeGraph.ragSelectRelated")}: ${otherName}`} onClick={() => onSelect(otherId)}>{otherName}</Button>
        <details className="mt-1"><summary className="cursor-pointer text-muted-foreground">{t("knowledgeGraph.ragSupportCount", { count: edge.referenceIds.length })}</summary>
          {references.filter((reference) => edge.referenceIds.includes(reference.id)).map((reference) => <article className="mt-2 space-y-1 border-l-2 pl-3" key={reference.id}>
            <span className="break-words font-medium">{reference.fileName ?? reference.sourceCode}</span>
            {reference.quote ? <blockquote className="whitespace-pre-wrap break-words">{reference.quote}</blockquote> : null}
          </article>)}
        </details>
      </div>;
    })}</section>
    <section className="space-y-3" aria-label={t("knowledgeGraph.ragSources")}><h3 className="font-medium">{t("knowledgeGraph.ragSources")}</h3>{sources.map((source) => <article className="space-y-2 border-b pb-3" key={source.id}>
      {source.documentId ? <Button className="h-auto justify-start whitespace-normal px-0 text-left" variant="link" onClick={() => navigate(`/settings/knowledge-base/${encodeURIComponent(source.corpusId)}?documentId=${encodeURIComponent(source.documentId!)}`)}>{source.fileName ?? source.sourceCode}</Button> : <p className="break-words font-medium">{source.fileName ?? source.sourceCode}</p>}
      <p className="text-sm text-muted-foreground">{Object.entries(source.location).filter(([key, value]) => /^(page|slide|sheet|paragraph|heading|row|cell|section)/i.test(key) && (typeof value === "string" || typeof value === "number" || Array.isArray(value))).map(([key, value]) => `${t(`ragUnified.locations.${key}`, { defaultValue: key })}: ${Array.isArray(value) ? value.join(" / ") : String(value)}`).join(" · ")}</p>
      {source.quote ? <blockquote className="whitespace-pre-wrap break-words border-l-2 pl-3 text-sm">{source.quote}</blockquote> : null}
    </article>)}</section>
  </InspectorPanel>;
}
