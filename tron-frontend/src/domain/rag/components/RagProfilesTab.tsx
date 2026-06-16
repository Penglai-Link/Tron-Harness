import {
  type KeyboardEvent,
  useEffect,
  useState,
} from "react";
import { useNavigate, useSearchParams } from "react-router";
import {
  FolderSync,
  Search,
} from "lucide-react";

import { SurfaceCard } from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";

import type {
  IngestProfileRecord,
  ProfileRecord,
  RagProfileType,
  RetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";
import {
  Pill,
  SectionTitle,
  statusTone,
} from "@/domain/rag/components/RagShared";

const profileTabs: Array<{ label: string; value: RagProfileType }> = [
  { label: "Ingest Profiles", value: "Ingest" },
  { label: "Retrieval Profiles", value: "Retrieval" },
];

const profileTypeStorageKey = "tron.rag.profiles.profileType";

function profileTypeFromValue(value: string | null): RagProfileType | null {
  const normalizedValue = value?.toLowerCase();
  if (normalizedValue === "retrieval") return "Retrieval";
  if (normalizedValue === "ingest") return "Ingest";
  return null;
}

function rememberProfileType(value: RagProfileType) {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(profileTypeStorageKey, profileTypeToSearch(value));
  } catch {
    // URL state still handles the normal path if browser storage is unavailable.
  }
}

function readRememberedProfileType(): RagProfileType {
  if (typeof window === "undefined") return "Ingest";
  try {
    return profileTypeFromValue(window.sessionStorage.getItem(profileTypeStorageKey)) ?? "Ingest";
  } catch {
    return "Ingest";
  }
}

function profileTypeToSearch(value: RagProfileType) {
  return value === "Retrieval" ? "retrieval" : "ingest";
}

export function RagProfilesTab({
  profiles,
}: {
  profiles: ProfileRecord[];
}) {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedProfileType = profileTypeFromValue(searchParams.get("profileType"));
  const [rememberedProfileType, setRememberedProfileType] = useState<RagProfileType>(() => readRememberedProfileType());
  const activeProfileType = requestedProfileType ?? rememberedProfileType;
  const ingestProfiles = profiles.filter((profile): profile is IngestProfileRecord => profile.type === "Ingest");
  const retrievalProfiles = profiles.filter((profile): profile is RetrievalProfileRecord => profile.type === "Retrieval");
  const visibleProfiles = activeProfileType === "Ingest" ? ingestProfiles : retrievalProfiles;
  const activeProfileTypeSearch = profileTypeToSearch(activeProfileType);
  const addPath = activeProfileType === "Ingest"
    ? `/rag/profiles/ingest/new?returnProfileType=${activeProfileTypeSearch}`
    : `/rag/profiles/retrieval/new?returnProfileType=${activeProfileTypeSearch}`;
  const addLabel = activeProfileType === "Ingest" ? "Add Ingest Profile" : "Add Retrieval Profile";
  const icon = activeProfileType === "Ingest"
    ? <FolderSync size={17} aria-hidden="true" />
    : <Search size={17} aria-hidden="true" />;

  useEffect(() => {
    if (!requestedProfileType) {
      const nextParams = new URLSearchParams(searchParams);
      nextParams.set("tab", "profiles");
      nextParams.set("profileType", profileTypeToSearch(rememberedProfileType));
      setSearchParams(nextParams, { replace: true });
      return;
    }
    rememberProfileType(requestedProfileType);
    setRememberedProfileType(requestedProfileType);
  }, [rememberedProfileType, requestedProfileType, searchParams, setSearchParams]);

  function setActiveProfileType(nextProfileType: RagProfileType) {
    rememberProfileType(nextProfileType);
    setRememberedProfileType(nextProfileType);
    const nextParams = new URLSearchParams(searchParams);
    nextParams.set("tab", "profiles");
    nextParams.set("profileType", profileTypeToSearch(nextProfileType));
    setSearchParams(nextParams, { replace: true });
  }

  function openAddProfile() {
    rememberProfileType(activeProfileType);
    navigate(addPath);
  }

  function openProfile(profile: ProfileRecord) {
    rememberProfileType(profile.type);
    navigate(profile.type === "Ingest"
      ? `/rag/profiles/ingest/${encodeURIComponent(profile.id)}/edit?returnProfileType=ingest`
      : `/rag/profiles/retrieval/${encodeURIComponent(profile.id)}/edit?returnProfileType=retrieval`);
  }

  return (
    <SurfaceCard>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <SectionTitle
          icon={icon}
          title="Profiles"
        />
        <Button onClick={openAddProfile}>
          {addLabel}
        </Button>
      </div>

      <div className="mt-5 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1">
        <div className="grid gap-1 sm:grid-cols-2">
          {profileTabs.map((tab) => {
            const selected = activeProfileType === tab.value;
            const count = tab.value === "Ingest" ? ingestProfiles.length : retrievalProfiles.length;
            return (
              <button
                aria-pressed={selected}
                className={`rounded-xl px-4 py-3 text-left transition ${
                  selected
                    ? "bg-[var(--color-elevated-surface)] shadow-sm"
                    : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface)]"
                }`}
                key={tab.value}
                type="button"
                onClick={() => setActiveProfileType(tab.value)}
              >
                <span className="flex items-center justify-between gap-3">
                  <span className="font-medium">{tab.label}</span>
                  <span className="rounded-full border border-[var(--color-outline-variant)] px-2 py-0.5 font-mono text-[12px]">
                    {count}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="mt-5 overflow-x-auto rounded-2xl border border-[var(--color-outline-variant)]">
        <div className="grid min-w-[780px] grid-cols-[minmax(220px,1.1fr)_minmax(180px,0.8fr)_minmax(220px,1fr)_minmax(160px,0.7fr)] gap-4 bg-[var(--color-surface-container-low)] px-4 py-3 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
          <span>Profile</span>
          <span>Corpus</span>
          <span>{activeProfileType === "Ingest" ? "Pipeline" : "Routes"}</span>
          <span>{activeProfileType === "Ingest" ? "Input Contract" : "Retrieval Policy"}</span>
        </div>

        {visibleProfiles.length > 0 ? visibleProfiles.map((profile) => (
          <ProfileRow
            key={profile.id}
            profile={profile}
            onEdit={() => openProfile(profile)}
          />
        )) : (
          <div className="min-w-[780px] px-4 py-8 text-center text-sm text-[var(--color-on-surface-variant)]">
            No {activeProfileType.toLowerCase()} profiles yet.
          </div>
        )}
      </div>
    </SurfaceCard>
  );
}

function ProfileRow({
  onEdit,
  profile,
}: {
  onEdit: () => void;
  profile: ProfileRecord;
}) {
  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onEdit();
    }
  }

  return (
    <div
      aria-label={`Edit ${profile.id}`}
      className="grid min-w-[780px] cursor-pointer grid-cols-[minmax(220px,1.1fr)_minmax(180px,0.8fr)_minmax(220px,1fr)_minmax(160px,0.7fr)] gap-4 border-t border-[var(--color-outline-variant)] px-4 py-4 text-sm hover:bg-[var(--color-hover-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-elevated-surface)]"
      role="button"
      tabIndex={0}
      onClick={onEdit}
      onKeyDown={handleKeyDown}
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <p className="truncate font-medium">{profile.id}</p>
          <Badge tone={statusTone(profile.status)}>{profile.status}</Badge>
        </div>
        {profile.description ? (
          <p className="mt-1 line-clamp-2 text-[13px] text-[var(--color-on-surface-variant)]">{profile.description}</p>
        ) : null}
      </div>

      <div className="grid content-start gap-2">
        <Pill>{profile.primaryCorpusId || "No primary corpus"}</Pill>
        {profile.type === "Retrieval" && profile.corpusIds.length > 1 ? (
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">
            {profile.corpusIds.length} corpora
          </span>
        ) : null}
      </div>

      {profile.type === "Ingest" ? <IngestProfileSummary profile={profile} /> : <RetrievalProfileSummary profile={profile} />}

      {profile.type === "Ingest" ? <IngestContractSummary profile={profile} /> : <RetrievalPolicySummary profile={profile} />}
    </div>
  );
}

function IngestProfileSummary({ profile }: { profile: IngestProfileRecord }) {
  const enabledStages = profile.config.dag.nodes;
  const payloadStrategy = profile.config.stages.preparePayload?.strategyId || profile.strategy || "Not set";
  return (
    <div className="grid content-start gap-2">
      <span className="font-mono text-[12px]">{payloadStrategy}</span>
      <span className="text-[12px] text-[var(--color-on-surface-variant)]">
        {enabledStages.length} stages · {profile.config.dag.edges.length} edges
      </span>
      <div className="flex flex-wrap gap-1.5">
        {enabledStages.slice(0, 4).map((stage) => <Pill key={stage}>{stage}</Pill>)}
      </div>
    </div>
  );
}

function RetrievalProfileSummary({ profile }: { profile: RetrievalProfileRecord }) {
  const enabledRoutes = profile.config.routesConfig.filter((route) => route.enabled);
  return (
    <div className="grid content-start gap-2">
      <span className="text-[12px] text-[var(--color-on-surface-variant)]">
        {enabledRoutes.length} enabled routes
      </span>
      <div className="flex flex-wrap gap-1.5">
        {enabledRoutes.slice(0, 4).map((route) => (
          <Pill key={`${route.corpusId}-${route.routeType}-${route.strategyId}`}>{route.routeType}</Pill>
        ))}
      </div>
    </div>
  );
}

function IngestContractSummary({ profile }: { profile: IngestProfileRecord }) {
  const sourceTypes = profile.config.sourceInputPolicy.supportedSourceTypes;
  return (
    <div className="grid content-start gap-1.5 text-[13px]">
      <span>Source version {profile.config.requiredSourceVersion || "Not set"}</span>
      <span className="text-[var(--color-on-surface-variant)]">
        {sourceTypes.length > 0 ? sourceTypes.join(", ") : "No source type"}
      </span>
    </div>
  );
}

function RetrievalPolicySummary({ profile }: { profile: RetrievalProfileRecord }) {
  return (
    <div className="grid content-start gap-1.5 text-[13px]">
      <span>{profile.config.fusionPolicy || "No fusion"}</span>
      <span className="text-[var(--color-on-surface-variant)]">
        {profile.config.contextStrategies.length} context strategies
      </span>
    </div>
  );
}
