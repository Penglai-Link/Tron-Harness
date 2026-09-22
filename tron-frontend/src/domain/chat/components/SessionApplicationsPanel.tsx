import { ChevronDown, ChevronRight, Folder, GitBranch, GitFork, Laptop, RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useSessionApplications } from "@/domain/chat/hooks/useSessionApplications";
import type { ChatSession } from "@/domain/chat/model";

export function SessionApplicationsPanel({ session, active }: { session: ChatSession; active: boolean }) {
  const { t } = useTranslation("pages");
  const [expandedApps, setExpandedApps] = useState<Record<string, boolean>>({});
  const query = useSessionApplications(session.sessionCode, active, `${session.lastRunCode ?? ""}:${session.lastRunStatus ?? ""}`);
  if (!session.projectCode) return null;
  return (
    <section className="mt-5 text-sm" data-testid="session-applications">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h3 className="font-medium">{t("chat.applications.title")}</h3>
        <Button variant="ghost" size="icon-sm" onClick={query.retry} disabled={query.loading} aria-label={t("chat.applications.refresh")}>
          <RefreshCw className={query.loading ? "size-4 animate-spin" : "size-4"} />
        </Button>
      </div>
      {query.error ? <p role="alert" className="text-[var(--color-error)]">{t("chat.applications.loadFailed")}</p> : query.loading && query.data.applications.length === 0 ? <p role="status">{t("chat.applications.loading")}</p> : query.data.applications.length === 0 ? <p className="text-[var(--color-on-surface-variant)]">{t("chat.applications.empty")}</p> : (
        <div className="divide-y divide-[var(--color-outline-variant)]">
          {[...query.data.applications].sort((left, right) => Number(right.isPrimary) - Number(left.isPrimary)).map((app) => {
            const appKey = `${app.appCode}:${app.configuredDirectory}`;
            const expanded = expandedApps[appKey] === true;
            const branch = app.branchName ?? (app.status === "Ready" && app.headCommit ? t("chat.applications.detached", { commit: app.headCommit.slice(0, 8) }) : t("chat.applications.unknown"));
            const kind = app.status === "MissingDirectory" || app.status === "Unavailable" ? t("chat.applications.unknown") : app.isWorktree ? t("chat.applications.worktree") : t("chat.applications.local");
            return (
            <article key={appKey} className="border-b border-[var(--color-outline-variant)] py-3 first:pt-1 last:border-b-0" data-app-code={app.appCode}>
              <button
                type="button"
                className="flex w-full min-w-0 items-start gap-2 text-left"
                aria-expanded={expanded}
                onClick={() => setExpandedApps((current) => ({ ...current, [appKey]: !expanded }))}
              >
                {expanded ? <ChevronDown className="mt-0.5 size-4 shrink-0" aria-hidden /> : <ChevronRight className="mt-0.5 size-4 shrink-0" aria-hidden />}
                <div className="min-w-0">
                  <h4 className="break-words font-medium">{app.appName}</h4>
                  <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-xs text-[var(--color-on-surface-variant)]">
                    <span className="inline-flex min-w-0 items-center gap-1"><GitBranch className="size-3.5 shrink-0" aria-hidden /><span className="break-all">{branch}</span></span>
                    <span className="inline-flex items-center gap-1">{app.isWorktree ? <GitFork className="size-3.5 shrink-0" aria-hidden /> : <Laptop className="size-3.5 shrink-0" aria-hidden />}{kind}</span>
                  </div>
                </div>
                {app.isPrimary ? <span className="ml-auto shrink-0 rounded bg-[var(--color-surface-container)] px-2 py-0.5 text-xs">{t("chat.applications.primary")}</span> : null}
              </button>
              {expanded ? <div className="mt-3 space-y-2 pl-6">
                <div className="flex items-start gap-2">
                  <Folder className="mt-0.5 size-4 shrink-0" aria-hidden />
                  <div className="min-w-0">
                    <p className="text-xs text-[var(--color-on-surface-variant)]">{t("chat.applications.directory")}</p>
                    <p className="break-all font-mono text-xs">{app.workingDirectory ?? app.configuredDirectory}</p>
                  </div>
                </div>
                {app.isWorktree && app.worktreePath && app.worktreePath !== app.workingDirectory ? <p className="break-all text-xs text-[var(--color-on-surface-variant)]">{t("chat.applications.worktreePath")}: {app.worktreePath}</p> : null}
                {app.status !== "Ready" ? <p className="text-xs text-[var(--color-on-surface-variant)]">{t(`chat.applications.status.${app.status}`)}</p> : null}
              </div> : null}
            </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
