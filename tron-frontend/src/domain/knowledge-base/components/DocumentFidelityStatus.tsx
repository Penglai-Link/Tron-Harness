import { useTranslation } from "react-i18next";
import type { FidelityIssue } from "../model/KnowledgeLibraryModel";

export function DocumentFidelityStatus({ issues, reviewRequired = false }: { issues?: FidelityIssue[]; reviewRequired?: boolean }) {
  const { t } = useTranslation("pages");
  if (!reviewRequired && !issues?.length) return null;
  return <section role="alert" className="mt-2 space-y-2 text-sm text-[var(--color-error)]">
    <p className="font-medium">{t("knowledgeLibrary.fidelity.title")}</p>
    <p>{t("knowledgeLibrary.fidelity.help")}</p>
    <p className="text-xs text-[var(--color-on-surface-variant)]">{t("knowledgeLibrary.fidelity.scope")}</p>
    {issues?.length ? <details><summary className="cursor-pointer">{t("knowledgeLibrary.fidelity.sources", { count: issues.length })}</summary>
      <ul className="mt-2 space-y-2">{issues.map((issue, index) => <li key={index} className="rounded border border-[var(--color-card-border)] p-2">
        <p>{issue.page ? t("knowledgeLibrary.location.page", { value: issue.page }) : t("knowledgeLibrary.fidelity.documentSource")} · {t(`knowledgeLibrary.fidelity.kinds.${issue.kind}`, { defaultValue: t("knowledgeLibrary.fidelity.unknown") })}</p>
        {issue.blocks.length ? <p className="text-xs">{t("knowledgeLibrary.fidelity.blocks", { value: issue.blocks.join(", ") })}</p> : null}
        {issue.candidateExcerpt ? <p className="text-xs">{t("knowledgeLibrary.fidelity.originalText")}</p> : null}
        <pre className="mt-1 max-h-24 overflow-auto whitespace-pre-wrap break-words font-sans text-xs">{issue.sourceExcerpt}</pre>
        {issue.candidateExcerpt ? <><p className="text-xs">{t("knowledgeLibrary.fidelity.candidateText")}</p><pre className="max-h-24 overflow-auto whitespace-pre-wrap break-words font-sans text-xs">{issue.candidateExcerpt}</pre></> : null}
      </li>)}</ul>
    </details> : null}
  </section>;
}
