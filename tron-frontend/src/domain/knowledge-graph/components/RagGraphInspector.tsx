import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { RagGraphNode, RagGraphEdge, RagGraphReference, RagGraphDirection } from "../model/RagGraphModel";

export interface RagGraphInspectorProps {
  ragNode: RagGraphNode; ragNodes: RagGraphNode[]; ragEdges: RagGraphEdge[]; ragReferences: RagGraphReference[];
  onClose: () => void; onExpand: (direction: RagGraphDirection) => void; onSelect: (id: string) => void;
  pending: boolean; incomingDepth: number; outgoingDepth: number;
}
export function RagGraphInspector({ ragNode: node, ragNodes: nodes, ragEdges: edges, ragReferences: references, onClose, onExpand, onSelect, pending, incomingDepth, outgoingDepth }: RagGraphInspectorProps) {
  const { t } = useTranslation("pages"); const navigate = useNavigate();
  const sourceIds = new Set([...node.referenceIds, ...edges.flatMap((edge) => edge.referenceIds)]);
  const sources = Array.from(new Map(references.filter(reference => sourceIds.has(reference.id)).map(reference => [
    JSON.stringify([reference.corpusId, reference.sourceCode, reference.sourceVersion, reference.documentId, reference.fileName,
      Object.entries(reference.location).sort(([left], [right]) => left.localeCompare(right)), reference.quote]),
    reference,
  ])).values());
  const names = new Map(nodes.map((item) => [item.id, item.name]));
  const texts = (items: { text: string }[]) => Array.from(new Set(items.map(item => item.text.trim()).filter(text => text && text !== node.name.trim())));
  const descriptions = texts(node.descriptions);
  const aliases = texts(node.aliases);
  return <section className="space-y-4 p-4" aria-label={t("knowledgeGraph.ragInspector")}>
    <div className="flex items-start justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4"><div className="min-w-0 space-y-2"><h2 className="break-words text-lg font-semibold">{node.name}</h2><Badge variant="outline">{node.entityType || t(`knowledgeGraph.ragNodeKinds.${node.kind}`, { defaultValue: node.kind })}</Badge></div>
      <Button type="button" size="icon" variant="ghost" aria-label={t("knowledgeGraph.inspector.closeAria")} onClick={onClose}><X aria-hidden="true" size={16} /></Button></div>
    {descriptions.map(description => <p className="whitespace-pre-wrap break-words text-sm leading-relaxed" key={description}>{description}</p>)}
    {aliases.length ? <p className="break-words text-sm text-muted-foreground">{t("knowledgeGraph.ragAliases")}: {aliases.join(", ")}</p> : null}
    <div className="flex flex-wrap gap-2"><Button type="button" variant="outline" disabled={pending || incomingDepth === 0} onClick={() => onExpand("Incoming")}>{t("knowledgeGraph.ragExpandIncoming")}</Button><Button type="button" variant="outline" disabled={pending || outgoingDepth === 0} onClick={() => onExpand("Outgoing")}>{t("knowledgeGraph.ragExpandOutgoing")}</Button></div>
    {edges.length ? <section className="space-y-2" aria-label={t("knowledgeGraph.ragRelationships")}><h3 className="text-sm font-medium">{t("knowledgeGraph.ragRelationships")} <span className="ml-1 font-normal tabular-nums text-muted-foreground">{edges.length}</span></h3><div className="divide-y divide-[var(--color-outline-variant)]">{edges.map((edge) => {
      const otherId = edge.from === node.id ? edge.to : edge.from;
      const otherName = names.get(otherId) ?? otherId;
      const incoming = edge.to === node.id;
      return <Button type="button" variant="ghost" className="h-auto w-full justify-between gap-3 whitespace-normal px-0 py-2.5 text-left" key={edge.id}
        aria-label={`${t("knowledgeGraph.ragSelectRelated")}: ${otherName}`} title={`${names.get(edge.from) ?? edge.from} → ${edge.relationship} → ${names.get(edge.to) ?? edge.to}`} onClick={() => onSelect(otherId)}>
        <span className="min-w-0 break-words text-sm">{otherName}</span>
        <span className="max-w-[45%] shrink-0 break-words text-right text-xs text-muted-foreground">{incoming ? "← " : ""}{edge.relationship}{incoming ? "" : " →"}</span>
      </Button>;
    })}</div></section> : null}
    {sources.length ? <section className="space-y-3" aria-label={t("knowledgeGraph.ragSources")}><h3 className="text-sm font-medium">{t("knowledgeGraph.ragSources")} <span className="ml-1 font-normal tabular-nums text-muted-foreground">{sources.length}</span></h3>{sources.map((source) => {
      const location = Object.entries(source.location).filter(([key, value]) => /^(page|slide|sheet|paragraph|heading|row|cell|section)/i.test(key) && (typeof value === "string" || typeof value === "number" || Array.isArray(value))).map(([key, value]) => `${t(`ragUnified.locations.${key}`, { defaultValue: key })}: ${Array.isArray(value) ? value.join(" / ") : String(value)}`).join(" · ");
      return <article className="space-y-2 border-b border-[var(--color-outline-variant)] pb-3 last:border-0 last:pb-0" key={source.id}>
        {source.documentId ? <Button type="button" className="h-auto justify-start whitespace-normal px-0 text-left" variant="link" onClick={() => navigate(`/settings/knowledge-base/${encodeURIComponent(source.corpusId)}?documentId=${encodeURIComponent(source.documentId!)}`)}>{source.fileName ?? source.sourceCode}</Button> : <p className="break-words text-sm font-medium">{source.fileName ?? source.sourceCode}</p>}
        {location ? <p className="text-xs text-muted-foreground">{location}</p> : null}
        {source.quote.trim() && source.quote.trim() !== node.name.trim() ? <blockquote className="whitespace-pre-wrap break-words border-l-2 border-[var(--color-outline-variant)] pl-3 text-sm leading-relaxed">{source.quote}</blockquote> : null}
      </article>;
    })}</section> : null}
  </section>;
}
