import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Link2,
  Search,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DetailField,
  Field,
  PageHeader,
  SearchField,
  SurfaceCard,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  bindWorkspaceSkill,
  fetchRegistrySkills,
} from "@/domain/skill/api";
import type { RegistrySkillRecord } from "@/domain/skill/model";

function truncateDescription(value?: string | null, maxLength = 140) {
  if (!value) {
    return null;
  }
  const normalized = value.trim().replace(/\s+/g, " ");
  if (!normalized) {
    return null;
  }
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength).trimEnd()}...`;
}

function SearchInput({
  value,
  onChange,
  onSubmit,
  loading,
}: {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  loading: boolean;
}) {
  return (
    <div className="flex flex-col gap-3 sm:flex-row">
      <SearchField
        ariaLabel="Search installed skills"
        className="flex-1"
        placeholder="Search installed skills..."
        value={value}
        onChange={onChange}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            onSubmit();
          }
        }}
      />
      <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={onSubmit}>
        <Search size={16} aria-hidden="true" />
        {loading ? "Searching..." : "Search"}
      </Button>
    </div>
  );
}

function CandidateCard({
  candidate,
  installed,
  selected,
  onToggle,
}: {
  candidate: RegistrySkillRecord;
  installed: boolean;
  selected: boolean;
  onToggle: () => void;
}) {
  const selectable = !installed;
  const description = truncateDescription(candidate.description);

  return (
    <label
      className={cn(
        "block w-full rounded-2xl border p-4 transition-colors",
        selectable ? "cursor-pointer" : "cursor-not-allowed bg-[var(--color-surface-container-low)] opacity-70",
        selected
          ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]"
          : "border-[var(--color-outline-variant)]",
        selectable ? "hover:bg-[var(--color-surface-container-low)]" : "",
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
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone="accent">{candidate.skillName}</Badge>
            {installed ? (
              <Badge tone="success">Installed</Badge>
            ) : null}
            <Badge tone="neutral">{candidate.sourceType}</Badge>
            {candidate.declaredVersion ? (
              <Badge tone="neutral">{candidate.declaredVersion}</Badge>
            ) : null}
          </div>
          <p className="mt-3 text-sm font-medium text-[var(--color-on-surface)]">
            {candidate.sourceRef}
          </p>
          {description ? (
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              {description}
            </p>
          ) : null}
        </div>
      </div>
    </label>
  );
}

export function BindSkillPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? "No workspace selected";
  const [query, setQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [binding, setBinding] = useState(false);
  const [searched, setSearched] = useState(false);
  const [candidates, setCandidates] = useState<RegistrySkillRecord[]>([]);
  const [selectedSkillNames, setSelectedSkillNames] = useState<string[]>([]);

  const selectedCandidates = useMemo(
    () => {
      const selectedSet = new Set(selectedSkillNames);
      return candidates.filter((candidate) => selectedSet.has(candidate.skillName));
    },
    [candidates, selectedSkillNames],
  );

  const selectedCount = selectedCandidates.length;

  function isInstalledCandidate(candidate: RegistrySkillRecord) {
    return candidate.enabled;
  }

  function toggleCandidate(skillName: string) {
    const candidate = candidates.find((item) => item.skillName === skillName);
    if (!candidate || isInstalledCandidate(candidate)) {
      return;
    }
    setSelectedSkillNames((current) =>
      current.includes(skillName)
        ? current.filter((value) => value !== skillName)
        : [...current, skillName],
    );
  }

  async function handleSearch(options?: { silentEmpty?: boolean }) {
    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before installing skills");
      return;
    }

    setSearching(true);
    setSearched(true);
    try {
      const results = await fetchRegistrySkills({
        search: query.trim() || undefined,
        workspaceCode: activeWorkspaceCode,
      });
      setCandidates(results);
      setSelectedSkillNames((current) => {
        const nextSet = new Set(results.map((candidate) => candidate.skillName));
        return current.filter((value) => nextSet.has(value));
      });
      if (!results.length && !options?.silentEmpty) {
        toast.info("No installed skills matched the current query");
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Installed skills could not be loaded";
      toast.error(message);
      setCandidates([]);
      setSelectedSkillNames([]);
    } finally {
      setSearching(false);
    }
  }

  useEffect(() => {
    if (!activeWorkspaceCode) {
      setSearched(false);
      setCandidates([]);
      setSelectedSkillNames([]);
      return;
    }
    void handleSearch({ silentEmpty: true });
  }, [activeWorkspaceCode]);

  async function handleBind() {
    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before installing skills");
      return;
    }
    if (!selectedCandidates.length || binding) return;

    setBinding(true);
    try {
      const failures: string[] = [];

      for (const candidate of selectedCandidates) {
        try {
          await bindWorkspaceSkill(activeWorkspaceCode, candidate.skillName, true);
        } catch (error) {
          const message =
            error instanceof Error ? error.message : "Skill install failed";
          failures.push(`${candidate.skillName}: ${message}`);
        }
      }

      const boundCount = selectedCandidates.length - failures.length;
      if (!failures.length) {
        toast.success(`${boundCount} skill${boundCount > 1 ? "s" : ""} installed in ${activeWorkspaceCode}`);
        navigate("/skills");
        return;
      }

      if (boundCount > 0) {
        toast.error(`Installed ${boundCount} of ${selectedCandidates.length} skills. ${failures[0]}`);
        return;
      }

      toast.error(failures[0] ?? "Skill install failed");
    } finally {
      setBinding(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Skills", href: "/skills" }, { label: "Install" }]}
        title="Install Skills"
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/skills")}>
              Cancel
            </Button>
            <Button
              disabled={!selectedCount || binding || !activeWorkspaceCode}
              aria-busy={binding}
              onClick={() => void handleBind()}
            >
              <Link2 size={16} aria-hidden="true" />
              {binding ? "Installing..." : `Install Selected${selectedCount ? ` (${selectedCount})` : ""}`}
            </Button>
          </>
        )}
      />

      <div className="space-y-6">
        <SurfaceCard className="space-y-4">
          <Field label="Skill Center">
            <SearchInput
              value={query}
              onChange={setQuery}
              onSubmit={() => void handleSearch()}
              loading={searching}
            />
          </Field>
        </SurfaceCard>

        <div className="grid gap-5 xl:grid-cols-2">
          <SurfaceCard className="space-y-4">
            <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
              Skill Center
            </h2>

            {!searched ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                Search synced skills in the local registry, then select the ones to install into this workspace.
              </div>
            ) : !candidates.length ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                No synced skills matched the current query.
              </div>
            ) : (
              <div className="space-y-3">
                {candidates.map((candidate) => (
                  <CandidateCard
                    key={candidate.skillName}
                    candidate={candidate}
                    installed={isInstalledCandidate(candidate)}
                    selected={selectedSkillNames.includes(candidate.skillName)}
                    onToggle={() => toggleCandidate(candidate.skillName)}
                  />
                ))}
              </div>
            )}
          </SurfaceCard>

          <SurfaceCard className="space-y-5">
            <h2 className="text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
              Install Plan
            </h2>

            {!selectedCount ? (
              <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
                No skills are selected yet.
              </div>
            ) : (
              <>
                <div className="grid gap-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                  <DetailField label="Workspace" value={activeWorkspaceLabel} />
                  <DetailField label="Selected" value={`${selectedCount} skill${selectedCount > 1 ? "s" : ""}`} />
                </div>

                <div className="space-y-3">
                  {selectedCandidates.map((candidate) => (
                    <div
                      key={candidate.skillName}
                      className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4"
                    >
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge tone="accent">{candidate.skillName}</Badge>
                        <Badge tone="neutral">{candidate.sourceType}</Badge>
                        {candidate.declaredVersion ? (
                          <Badge tone="neutral">{candidate.declaredVersion}</Badge>
                        ) : null}
                      </div>
                      <p className="mt-3 break-words text-sm font-medium text-[var(--color-on-surface)]">
                        {candidate.sourceRef}
                      </p>
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
