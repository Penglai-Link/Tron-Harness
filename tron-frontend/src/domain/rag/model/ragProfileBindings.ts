import type {
  CorpusRecord,
  IngestProfileRecord,
} from "@/domain/rag/model/RagModel";

const preferredIngestProfileByCorpus: Record<string, string> = {
  "memory-card": "memory-card-ingest-v1",
  "project-doc": "project-doc-mineru-ingest-v1",
  skill: "skill-ingest-v1",
  "skill-graph": "skill-graph-ingest-v1",
};

function referencesCorpus(profile: IngestProfileRecord, corpusCode: string) {
  return profile.primaryCorpusId === corpusCode
    || profile.corpus === corpusCode
    || profile.corpusIds.includes(corpusCode);
}

function profilePreference(profile: IngestProfileRecord, corpusCode: string) {
  const preferredProfileId = preferredIngestProfileByCorpus[corpusCode];
  return [
    preferredProfileId && profile.id !== preferredProfileId ? 1 : 0,
    profile.primaryCorpusId === corpusCode ? 0 : 1,
    profile.corpusIds.length,
    profile.id,
  ] as const;
}

function compareProfilePreference(left: IngestProfileRecord, right: IngestProfileRecord, corpusCode: string) {
  const leftPreference = profilePreference(left, corpusCode);
  const rightPreference = profilePreference(right, corpusCode);
  for (let index = 0; index < leftPreference.length; index += 1) {
    const leftValue = leftPreference[index];
    const rightValue = rightPreference[index];
    if (leftValue < rightValue) return -1;
    if (leftValue > rightValue) return 1;
  }
  return 0;
}

export function resolvePreferredIngestProfile(profiles: IngestProfileRecord[], corpusCode: string) {
  return profiles
    .filter((candidate) => referencesCorpus(candidate, corpusCode))
    .sort((left, right) => compareProfilePreference(left, right, corpusCode))[0];
}

export function resolveCorpusIngestProfiles(corpora: CorpusRecord[], profiles: IngestProfileRecord[]) {
  if (profiles.length === 0) return corpora;
  return corpora.map((corpus) => {
    if (corpus.ingestProfile.trim()) return corpus;
    const profile = resolvePreferredIngestProfile(profiles, corpus.code);
    return profile ? { ...corpus, ingestProfile: profile.id } : corpus;
  });
}
