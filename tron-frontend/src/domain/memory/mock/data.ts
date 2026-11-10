import type { MemoryWorkDomainRecord } from "../api";
let memoryWorkDomains: MemoryWorkDomainRecord[] = [
  {
    createdAt: "2026-06-12 10:00",
    description:
      "UI implementation rules, design-system decisions, responsive behavior, and browser workflow memory.",
    domainCode: "frontend",
    name: "Frontend Engineering",
    status: "ACTIVE",
    updatedAt: "2026-06-17 09:20",
  },
  {
    createdAt: "2026-06-12 10:00",
    description:
      "Rust service contracts, migrations, storage behavior, and API integration memory.",
    domainCode: "backend",
    name: "Backend Engineering",
    status: "ACTIVE",
    updatedAt: "2026-06-16 14:10",
  },
  {
    createdAt: "2026-06-12 10:00",
    description:
      "Verification strategy, focused checks, Playwright scenarios, and release quality memory.",
    domainCode: "testing",
    name: "Testing",
    status: "ACTIVE",
    updatedAt: "2026-06-14 11:05",
  },
  {
    createdAt: "2026-06-12 10:00",
    description:
      "Product behavior, acceptance criteria, UX decisions, and rollout planning memory.",
    domainCode: "product",
    name: "Product",
    status: "ACTIVE",
    updatedAt: "2026-06-13 18:24",
  },
  {
    createdAt: "2026-06-12 10:00",
    description:
      "Default work-domain target used when a more specific domain is not selected.",
    domainCode: "main-domain",
    name: "Main Domain",
    status: "ACTIVE",
    updatedAt: "2026-06-12 12:30",
  },
  {
    createdAt: "2026-06-11 09:15",
    description:
      "Archived mobile-specific decisions retained for historical lookup only.",
    domainCode: "legacy-mobile",
    name: "Legacy Mobile",
    status: "DISABLED",
    updatedAt: "2026-06-08 16:42",
  },
];
function formatTimestamp() {
  return new Date().toISOString();
}
export function listMemoryWorkDomainsMock(
  options: { includeDisabled?: boolean; search?: string } = {},
) {
  const includeDisabled = options.includeDisabled ?? false;
  const normalizedQuery = (options.search ?? "").trim().toLowerCase();

  return memoryWorkDomains.filter((domain) => {
    if (!includeDisabled && domain.status === "DISABLED") return false;
    if (!normalizedQuery) return true;

    return [
      domain.domainCode,
      domain.name,
      domain.description ?? "",
      domain.status,
    ].some((value) => value.toLowerCase().includes(normalizedQuery));
  });
}

export function createMemoryWorkDomainRecord(input: {
  description?: string | null;
  domainCode: string;
  name: string;
}) {
  const domainCode = input.domainCode.trim();
  if (memoryWorkDomains.some((domain) => domain.domainCode === domainCode)) {
    throw new Error("Memory domain already exists");
  }

  const record: MemoryWorkDomainRecord = {
    createdAt: formatTimestamp(),
    description: input.description?.trim() || null,
    domainCode,
    name: input.name.trim(),
    status: "ACTIVE",
    updatedAt: formatTimestamp(),
  };

  memoryWorkDomains = [record, ...memoryWorkDomains];
  return record;
}

export function updateMemoryWorkDomainRecord(
  domainCode: string,
  input: {
    description?: string | null;
    name?: string;
    status?: MemoryWorkDomainRecord["status"];
  },
) {
  const current = memoryWorkDomains.find(
    (domain) => domain.domainCode === domainCode,
  );
  if (!current) {
    throw new Error("Memory domain not found");
  }

  const next: MemoryWorkDomainRecord = {
    ...current,
    description:
      input.description === undefined
        ? current.description
        : input.description?.trim() || null,
    name: input.name?.trim() || current.name,
    status: input.status ?? current.status,
    updatedAt: formatTimestamp(),
  };

  memoryWorkDomains = memoryWorkDomains.map((domain) =>
    domain.domainCode === domainCode ? next : domain,
  );
  return next;
}

export function enableMemoryWorkDomainRecord(domainCode: string) {
  return updateMemoryWorkDomainRecord(domainCode, { status: "ACTIVE" });
}

export function disableMemoryWorkDomainRecord(domainCode: string) {
  return updateMemoryWorkDomainRecord(domainCode, { status: "DISABLED" });
}
