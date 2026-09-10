import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { ExternalLink } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/components/ui/badge";
import { DetailField } from "@/components/detail-field";
import { Field, SelectInput } from "@/components/forms";
import { SearchField } from "@/components/search-field";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  fetchRegistrySkills,
  installRegistrySkill,
  searchMarketplaceSkills,
} from "@/domain/skill/api";
import type {
  RegistrySkillRecord,
  SkillSearchCandidate,
  SkillSearchMode,
} from "@/domain/skill/model";
import { skillIndexWarningMessage } from "@/domain/skill/utils/ragIndexWarning";
import { SelectItem } from "@/components/ui/select";
import { Button, buttonVariants } from "@/components/ui/button";

type InstallCandidateState = "pending" | "installing" | "success" | "error";

interface InstallProgressState {
  completedCount: number;
  currentSkillName?: string;
  failureCount: number;
  results: Record<string, { message?: string; state: InstallCandidateState }>;
  successCount: number;
  totalCount: number;
}

const searchModeOptions: Array<{
  mode: SkillSearchMode;
  label: string;
  fieldLabel: string;
  placeholder: string;
  emptyHint: string;
  buttonLabel: string;
}> = [
    {
      mode: "find",
      label: "Find",
      fieldLabel: "Marketplace Query",
      placeholder: "Search skills with npx skills find...",
      emptyHint:
        "Enter keywords like `typescript`, `rag`, or `review` to query the skills marketplace.",
      buttonLabel: "Search",
    },
    {
      mode: "repo",
      label: "Repo",
      fieldLabel: "Repository",
      placeholder: "owner/repo or git URL",
      emptyHint:
        "Enter a repository like `cloudsen/eliteforge-skills` to list its available skills.",
      buttonLabel: "List Skills",
    },
  ];

function SearchInput({
  mode,
  value,
  onChange,
  onModeChange,
  onSubmit,
  disabled,
  loading,
}: {
  mode: SkillSearchMode;
  value: string;
  onChange: (value: string) => void;
  onModeChange: (mode: SkillSearchMode) => void;
  onSubmit: () => void;
  disabled?: boolean;
  loading: boolean;
}) {
  const { t } = useTranslation(["pages"]);
  const activeOption =
    searchModeOptions.find((option) => option.mode === mode) ??
    searchModeOptions[0];

  return (
    <div className="flex gap-[12px]">
      <Field label>
        <SelectInput
          aria-label="Runtime role"
          disabled={disabled}
          value={mode}
          onValueChange={(event) => onModeChange(event as SkillSearchMode)}
        >
          {searchModeOptions.map((option) => (
            <SelectItem key={option.mode} value={option.mode}>
              {option.label}
            </SelectItem>
          ))}
        </SelectInput>
      </Field>
      <Field label>
        <SearchField
          ariaLabel={
            mode === "repo" ? "Repository source" : "Search skills marketplace"
          }
          className="flex-1"
          disabled={disabled}
          placeholder={activeOption.placeholder}
          value={value}
          onChange={onChange}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onSubmit();
            }
          }}
        />
      </Field>
      <Field label>
        <Button
          size='lg'
          disabled={disabled || loading || !value.trim()}
          aria-busy={loading}
          onClick={onSubmit}
        >
          {loading ? t("pages:skills.actions.searching", "Searching...") : t(`pages:skills.searchModes.${mode}.button`, activeOption.buttonLabel)}
        </Button>
      </Field>
    </div>
  );
}

function CandidateCard({
  candidate,
  disabled,
  installed,
  installState,
  selected,
  onToggle,
}: {
  candidate: SkillSearchCandidate;
  disabled?: boolean;
  installed: boolean;
  installState?: { message?: string; state: InstallCandidateState };
  selected: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation(["pages"]);
  const selectable = !installed && !disabled;

  return (
    <label
      className={cn(
        "block w-full rounded-2xl border p-4 transition-colors",
        selectable ? "cursor-pointer" : "cursor-not-allowed",
        selected
          ? "border-[var(--color-primary)] bg-[color-mix(in_srgb,var(--color-primary)_12%,var(--color-surface))] shadow-[inset_0_0_0_1px_var(--color-primary)]"
          : "border-[var(--color-outline-variant)]",
        selectable
          ? "hover:bg-[var(--color-surface-container-low)]"
          : "bg-[var(--color-surface-container-low)] opacity-70",
      )}
    >
      <div className="flex items-start gap-3">
        <input
          className="mt-1 size-4 rounded border-[var(--color-outline-variant)]"
          checked={selectable && selected}
          disabled={!selectable}
          aria-label={`Select ${candidate.skillName}`}
          type="checkbox"
          onChange={onToggle}
        />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-[12px]">
            <Badge variant="info">{candidate.skillName}</Badge>
            {installed ? <Badge variant="success">{t("pages:skills.status.installed", "Installed")}</Badge> : null}
            {installState?.state === "installing" ? (
              <Badge variant="warning">{t("pages:skills.status.installing", "Installing")}</Badge>
            ) : null}
            {installState?.state === "error" ? (
              <Badge variant="error">{t("pages:skills.status.failed", "Failed")}</Badge>
            ) : null}
            {candidate.installsLabel ? (
              <Badge variant="info">{candidate.installsLabel}</Badge>
            ) : null}
          </div>
          <p className="mt-3 text-sm font-medium text-[var(--color-on-surface)]">
            {candidate.packageRef}
          </p>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {candidate.sourceRef}
          </p>
          {candidate.description ? (
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              {candidate.description}
            </p>
          ) : null}
          {installState?.state === "error" && installState.message ? (
            <p className="mt-2 text-sm text-[var(--color-error)]">
              {installState.message}
            </p>
          ) : null}
        </div>
      </div>
    </label>
  );
}

export function CreateSkillPage() {
  const { t } = useTranslation(["pages"]);
  const navigate = useNavigate();
  const toast = useToast();
  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState<SkillSearchMode>("find");
  const [searching, setSearching] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [searched, setSearched] = useState(false);
  const [candidates, setCandidates] = useState<SkillSearchCandidate[]>([]);
  const [selectedPackageRefs, setSelectedPackageRefs] = useState<string[]>([]);
  const [installedSkills, setInstalledSkills] = useState<RegistrySkillRecord[]>(
    [],
  );
  const [installProgress, setInstallProgress] =
    useState<InstallProgressState | null>(null);
  const [lastInstallPlan, setLastInstallPlan] = useState<
    SkillSearchCandidate[]
  >([]);

  async function readInstalledSkills() {
    try {
      return await fetchRegistrySkills();
    } catch {
      return [];
    }
  }

  async function loadInstalledSkills() {
    const records = await readInstalledSkills();
    setInstalledSkills(records);
    return records;
  }

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const records = await readInstalledSkills();
      if (!cancelled) {
        setInstalledSkills(records);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const activeSearchMode =
    searchModeOptions.find((option) => option.mode === searchMode) ??
    searchModeOptions[0];
  const installedPackageRefs = useMemo(
    () =>
      new Set(
        installedSkills.map((skill) => `${skill.sourceRef}@${skill.skillName}`),
      ),
    [installedSkills],
  );
  const installedSkillNames = useMemo(
    () => new Set(installedSkills.map((skill) => skill.skillName)),
    [installedSkills],
  );

  const selectedCandidates = useMemo(() => {
    const selectedSet = new Set(selectedPackageRefs);
    return candidates.filter((candidate) =>
      selectedSet.has(candidate.packageRef),
    );
  }, [candidates, selectedPackageRefs]);

  const selectedCount = selectedCandidates.length;
  const planCandidates = selectedCandidates.length
    ? selectedCandidates
    : lastInstallPlan;

  function isInstalledCandidate(candidate: SkillSearchCandidate) {
    return (
      installedPackageRefs.has(candidate.packageRef) ||
      installedSkillNames.has(candidate.skillName)
    );
  }

  function toggleCandidate(packageRef: string) {
    const candidate = candidates.find((item) => item.packageRef === packageRef);
    if (!candidate || isInstalledCandidate(candidate)) {
      return;
    }
    setSelectedPackageRefs((current) =>
      current.includes(packageRef)
        ? current.filter((value) => value !== packageRef)
        : [...current, packageRef],
    );
  }

  function handleModeChange(mode: SkillSearchMode) {
    if (mode === searchMode) {
      return;
    }
    setSearchMode(mode);
    setSearched(false);
    setCandidates([]);
    setSelectedPackageRefs([]);
    setInstallProgress(null);
    setLastInstallPlan([]);
  }

  async function handleSearch() {
    if (!query.trim() || searching) return;
    setSearching(true);
    setSearched(true);
    setCandidates([]);
    setSelectedPackageRefs([]);
    setInstallProgress(null);
    setLastInstallPlan([]);
    try {
      const results = await searchMarketplaceSkills(query.trim(), searchMode);
      setCandidates(results);
      if (!results.length) {
        toast.info(t("pages:skills.messages.noMatch", "No skills matched the current query"));
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Skills search failed";
      toast.error(message);
      setCandidates([]);
      setSelectedPackageRefs([]);
    } finally {
      setSearching(false);
    }
  }

  async function handleInstall() {
    if (!selectedCandidates.length || installing) return;
    const installQueue = [...selectedCandidates];
    const initialResults = Object.fromEntries(
      installQueue.map((candidate) => [
        candidate.packageRef,
        { state: "pending" as InstallCandidateState },
      ]),
    );
    setInstalling(true);
    setLastInstallPlan(installQueue);
    setInstallProgress({
      completedCount: 0,
      currentSkillName: installQueue[0]?.skillName,
      failureCount: 0,
      results: initialResults,
      successCount: 0,
      totalCount: installQueue.length,
    });
    try {
      const failures: Array<{
        message: string;
        packageRef: string;
        skillName: string;
      }> = [];
      const warnings: Array<{ message: string; skillName: string }> = [];
      const failedPackageRefs: string[] = [];
      let successCount = 0;
      let failureCount = 0;

      for (const [index, candidate] of installQueue.entries()) {
        setInstallProgress((current) => ({
          completedCount: current?.completedCount ?? 0,
          currentSkillName: candidate.skillName,
          failureCount: current?.failureCount ?? 0,
          results: {
            ...(current?.results ?? {}),
            [candidate.packageRef]: { state: "installing" },
          },
          successCount: current?.successCount ?? 0,
          totalCount: current?.totalCount ?? installQueue.length,
        }));
        try {
          const result = await installRegistrySkill({
            packageRef: candidate.packageRef,
            skillName: candidate.skillName,
            sourceRef: candidate.sourceRef,
          });
          const warning = skillIndexWarningMessage(result.ragIndex);
          if (warning) {
            warnings.push({ message: warning, skillName: candidate.skillName });
          }
          successCount += 1;
          setInstallProgress((current) => ({
            completedCount: index + 1,
            currentSkillName:
              index + 1 < installQueue.length
                ? installQueue[index + 1]?.skillName
                : undefined,
            failureCount,
            results: {
              ...(current?.results ?? {}),
              [candidate.packageRef]: { state: "success" },
            },
            successCount,
            totalCount: current?.totalCount ?? installQueue.length,
          }));
        } catch (error) {
          const message =
            error instanceof Error
              ? error.message
              : "Skill installation failed";
          failures.push({
            message,
            packageRef: candidate.packageRef,
            skillName: candidate.skillName,
          });
          failedPackageRefs.push(candidate.packageRef);
          failureCount += 1;
          setInstallProgress((current) => ({
            completedCount: index + 1,
            currentSkillName:
              index + 1 < installQueue.length
                ? installQueue[index + 1]?.skillName
                : undefined,
            failureCount,
            results: {
              ...(current?.results ?? {}),
              [candidate.packageRef]: { message, state: "error" },
            },
            successCount,
            totalCount: current?.totalCount ?? installQueue.length,
          }));
        }
      }

      await loadInstalledSkills();

      const installedCount = installQueue.length - failures.length;
      if (!failures.length) {
        if (warnings.length) {
          toast.warning(`${warnings[0]?.skillName}: ${warnings[0]?.message}`);
        } else {
          toast.success(
            `${installedCount} skill${installedCount > 1 ? "s" : ""} installed`,
          );
        }
        setSelectedPackageRefs([]);
        return;
      }

      if (installedCount > 0) {
        setSelectedPackageRefs(failedPackageRefs);
        toast.warning(
          `Installed ${installedCount} of ${installQueue.length} skills. ${failures[0]?.skillName}: ${failures[0]?.message}`,
        );
        return;
      }

      setSelectedPackageRefs(failedPackageRefs);
      const firstFailure = failures[0];
      toast.error(
        firstFailure
          ? `${firstFailure.skillName}: ${firstFailure.message}`
          : "Skill installation failed",
      );
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Skill installation failed";
      toast.error(message);
    } finally {
      setInstalling(false);
      setInstallProgress((current) =>
        current
          ? {
            ...current,
            currentSkillName: undefined,
          }
          : null,
      );
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: "Skill Center", href: "/settings/skills" },
          { label: "Sync" },
        ]}
        title={t("pages:skills.syncTitle", "Sync Skills")}
        actions={
          <>
            <Button
              size='lg'
              variant="outline"
              disabled={installing}
              onClick={() => navigate("/settings/skills")}
            >
              {t("pages:skills.actions.cancel", "Cancel")}
            </Button>
            <Button
              size='lg'
              disabled={!selectedCount || installing}
              aria-busy={installing}
              onClick={() => void handleInstall()}
            >
              {installing
                ? `Syncing ${installProgress?.completedCount ?? 0}/${installProgress?.totalCount ?? selectedCount
                }...`
                : t("pages:skills.actions.syncSelected", "Sync Selected{{suffix}}", { suffix: selectedCount ? ` (${selectedCount})` : "" })}
            </Button>
          </>
        }
      />

      <div className="space-y-6">
        <SurfaceCard>
          <Field label={t("pages:skills.fields.syncType", "Sync Type")}>
            <SearchInput
              mode={searchMode}
              value={query}
              onChange={setQuery}
              onModeChange={handleModeChange}
              onSubmit={() => void handleSearch()}
              disabled={installing}
              loading={searching}
            />
          </Field>
        </SurfaceCard>

        <div className="grid gap-5 xl:grid-cols-2">
          <SurfaceCard className="space-y-4">
            <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
              {t("pages:skills.sections.searchResults", "Search Results")}
            </h2>

            {!searched ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                {activeSearchMode.emptyHint}
              </div>
            ) : searching ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                {searchMode === "repo"
                  ? "Listing skills from the repository source..."
                  : "Searching marketplace skills..."}
              </div>
            ) : !candidates.length ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                {searchMode === "repo"
                  ? "No skills were discovered in the current repository source."
                  : "No marketplace skills matched the current query."}
              </div>
            ) : (
              <div className="space-y-3">
                {candidates.map((candidate) => (
                  <CandidateCard
                    key={candidate.packageRef}
                    candidate={candidate}
                    disabled={installing}
                    installed={isInstalledCandidate(candidate)}
                    installState={
                      installProgress?.results[candidate.packageRef]
                    }
                    selected={selectedPackageRefs.includes(
                      candidate.packageRef,
                    )}
                    onToggle={() => toggleCandidate(candidate.packageRef)}
                  />
                ))}
              </div>
            )}
          </SurfaceCard>

          <SurfaceCard className="space-y-5">
            <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
              {t("pages:skills.sections.syncPlan", "Sync Plan")}
            </h2>

            {!planCandidates.length ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                No skills are selected yet.
              </div>
            ) : (
              <>
                <div className="grid gap-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                  <DetailField
                    label="Selected"
                    value={`${planCandidates.length} skill${planCandidates.length > 1 ? "s" : ""
                      }`}
                  />
                  {installProgress ? (
                    <>
                      <DetailField
                        label="Progress"
                        value={`${installProgress.completedCount}/${installProgress.totalCount}`}
                      />
                      <DetailField
                        label="Succeeded"
                        value={`${installProgress.successCount}`}
                      />
                      <DetailField
                        label="Failed"
                        value={`${installProgress.failureCount}`}
                      />
                      <DetailField
                        label="Current"
                        value={installProgress.currentSkillName ?? "Completed"}
                      />
                    </>
                  ) : null}
                </div>

                {installProgress ? (
                  <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-4 py-3 text-sm text-[var(--color-on-surface-variant)]">
                    {installing
                      ? `Syncing ${installProgress.completedCount}/${installProgress.totalCount
                      }. ${installProgress.currentSkillName ??
                      "Waiting for next skill..."
                      }`
                      : installProgress.failureCount > 0
                        ? `Sync finished with ${installProgress.failureCount} failure(s). Failed skills remain selected so you can retry them.`
                        : "Sync finished successfully."}
                  </div>
                ) : null}

                <div className="space-y-3">
                  {planCandidates.map((candidate) => (
                    <div
                      key={candidate.packageRef}
                      className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-[12px]">
                            <Badge variant="info">{candidate.skillName}</Badge>
                            {isInstalledCandidate(candidate) ? (
                              <Badge variant="success">Installed</Badge>
                            ) : null}
                            {installProgress?.results[candidate.packageRef]
                              ?.state === "installing" ? (
                              <Badge variant="warning">Installing</Badge>
                            ) : null}
                            {installProgress?.results[candidate.packageRef]
                              ?.state === "error" ? (
                              <Badge variant="error">Failed</Badge>
                            ) : null}
                            {candidate.installsLabel ? (
                              <Badge variant="info">
                                {candidate.installsLabel}
                              </Badge>
                            ) : null}
                          </div>
                          <p className="mt-3 break-words text-sm font-medium text-[var(--color-on-surface)]">
                            {candidate.packageRef}
                          </p>
                          <p className="mt-1 break-words text-sm text-[var(--color-on-surface-variant)]">
                            {candidate.sourceRef}
                          </p>
                          {candidate.description ? (
                            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
                              {candidate.description}
                            </p>
                          ) : null}
                          {installProgress?.results[candidate.packageRef]
                            ?.state === "error" &&
                            installProgress.results[candidate.packageRef]
                              ?.message ? (
                            <p className="mt-2 text-sm text-[var(--color-error)]">
                              {
                                installProgress.results[candidate.packageRef]
                                  ?.message
                              }
                            </p>
                          ) : null}
                        </div>

                        {candidate.detailUrl ? (
                          <a
                            href={candidate.detailUrl}
                            rel="noreferrer"
                            target="_blank"
                            className={buttonVariants({ size: "sm", variant: "secondary" })}
                          >
                            <ExternalLink size={15} aria-hidden="true" />
                            Open
                          </a>
                        ) : null}
                      </div>
                    </div>
                  ))}
                </div>
              </>
            )}
          </SurfaceCard>
        </div>
      </div>
    </>
  );
}
