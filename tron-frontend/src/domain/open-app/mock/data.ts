import type {
  ManagedOpenAppDetail,
  ManagedOpenAppSecretRevealResult,
  ManagedOpenAppSummary,
  OpenApiCapabilityCatalogGroup,
  OpenAppStatus,
} from "@/domain/open-app/model";

import type {
  CreateOpenAppPayload,
  ReplaceOpenAppCapabilityGrantsPayload,
  ReplaceOpenAppUserGrantsPayload,
  UpdateOpenAppPayload,
} from "@/domain/open-app/api";

const capabilityCatalog: OpenApiCapabilityCatalogGroup[] = [
  {
    capabilityCode: "agent-chat",
    capabilityName: "Agent Chat OpenAPI",
    description: "External session and message access for agent chat.",
    operations: [
      {
        operationCode: "message.read",
        operationName: "Read Messages",
        description: "Read session message history.",
        routes: ["GET /open-api/v1/chats/sessions/{sessionCode}/messages"],
      },
      {
        operationCode: "message.write",
        operationName: "Write Messages",
        description: "Create session messages and receive model replies.",
        routes: ["POST /open-api/v1/chats/sessions/{sessionCode}/messages"],
      },
      {
        operationCode: "session.read",
        operationName: "Read Sessions",
        description: "Read session groups and AG-UI event feeds.",
        routes: [
          "GET /open-api/v1/chats/session-groups",
          "GET /open-api/v1/agents/agui/streams/{streamKey}/events",
          "GET /open-api/v1/agents/agui/streams/{streamKey}/protocol-events",
        ],
      },
      {
        operationCode: "session.write",
        operationName: "Write Sessions",
        description: "Create or update agent chat sessions.",
        routes: [
          "POST /open-api/v1/chats/sessions",
          "PATCH /open-api/v1/chats/sessions/{sessionCode}",
        ],
      },
      {
        operationCode: "stream.consume",
        operationName: "Consume Streams",
        description: "Consume streaming chat responses.",
        routes: ["POST /open-api/v1/chats/sessions/{sessionCode}/messages/stream"],
      },
    ],
  },
];

let mockOpenApps: ManagedOpenAppDetail[] = [
  {
    openAppCode: "OPENAPP-FIXTURE-0001",
    name: "Knowledge Bridge",
    description: "Knowledge-base handoff integration.",
    status: "Enabled",
    appKeyFingerprint: "7d12eddb5276",
    createdAt: "2026-06-11T13:40:12Z",
    updatedAt: "2026-06-11T13:40:12Z",
    lastRotatedAt: "2026-06-11T13:40:12Z",
    userGrants: [{ userCode: "USR-0001" }],
    capabilityGrants: [
      { capabilityCode: "agent-chat", operationCode: "session.read" },
      { capabilityCode: "agent-chat", operationCode: "message.write" },
    ],
  },
];

let mockSequence = 1;

function summarize(record: ManagedOpenAppDetail): ManagedOpenAppSummary {
  return {
    openAppCode: record.openAppCode,
    name: record.name,
    description: record.description,
    status: record.status,
    appKeyFingerprint: record.appKeyFingerprint,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
    lastRotatedAt: record.lastRotatedAt,
    userGrantCount: record.userGrants.length,
    capabilityGrantCount: record.capabilityGrants.length,
  };
}

function cloneDetail(record: ManagedOpenAppDetail): ManagedOpenAppDetail {
  return {
    ...record,
    userGrants: record.userGrants.map((grant) => ({ ...grant })),
    capabilityGrants: record.capabilityGrants.map((grant) => ({ ...grant })),
  };
}

function cloneCatalog(): OpenApiCapabilityCatalogGroup[] {
  return capabilityCatalog.map((group) => ({
    ...group,
    operations: group.operations.map((operation) => ({
      ...operation,
      routes: [...operation.routes],
    })),
  }));
}

function randomFingerprint() {
  return Math.random().toString(16).slice(2, 14).padEnd(12, "0");
}

function nowIso() {
  return new Date().toISOString();
}

function requireRecord(openAppCode: string) {
  const record = mockOpenApps.find((item) => item.openAppCode === openAppCode);
  if (!record) {
    throw new Error("open-app was not found");
  }
  return record;
}

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values.map((value) => value.trim()).filter(Boolean))).sort();
}

function isKnownGrant(capabilityCode: string, operationCode: string) {
  return capabilityCatalog.some((group) => (
    group.capabilityCode === capabilityCode
      && group.operations.some((operation) => operation.operationCode === operationCode)
  ));
}

export function listPlatformOpenAppsMock() {
  return mockOpenApps.map((record) => summarize(record));
}

export function listPlatformOpenAppCatalogMock() {
  return cloneCatalog();
}

export function getPlatformOpenAppMock(openAppCode: string) {
  return cloneDetail(requireRecord(openAppCode));
}

export function createPlatformOpenAppMock(
  payload: CreateOpenAppPayload,
): ManagedOpenAppSecretRevealResult {
  const now = nowIso();
  mockSequence += 1;
  const openAppCode = `OPENAPP-MOCK-${mockSequence.toString().padStart(4, "0")}`;
  const fingerprint = randomFingerprint();
  const record: ManagedOpenAppDetail = {
    openAppCode,
    name: payload.name.trim(),
    description: payload.description?.trim() || null,
    status: "Enabled",
    appKeyFingerprint: fingerprint,
    createdAt: now,
    updatedAt: now,
    lastRotatedAt: now,
    userGrants: [],
    capabilityGrants: [],
  };
  mockOpenApps = [record, ...mockOpenApps];
  return {
    openApp: cloneDetail(record),
    secretReveal: {
      openAppCode,
      appKey: `mock-open-app-key-${mockSequence}`,
      appKeyFingerprint: fingerprint,
    },
  };
}

export function updatePlatformOpenAppMock(
  openAppCode: string,
  payload: UpdateOpenAppPayload,
) {
  const record = requireRecord(openAppCode);
  record.name = payload.name.trim();
  record.description = payload.description?.trim() || null;
  record.status = payload.status as OpenAppStatus;
  record.updatedAt = nowIso();
  return cloneDetail(record);
}

export function rotatePlatformOpenAppKeyMock(openAppCode: string): ManagedOpenAppSecretRevealResult {
  const record = requireRecord(openAppCode);
  const fingerprint = randomFingerprint();
  const rotatedAt = nowIso();
  record.appKeyFingerprint = fingerprint;
  record.updatedAt = rotatedAt;
  record.lastRotatedAt = rotatedAt;
  return {
    openApp: cloneDetail(record),
    secretReveal: {
      openAppCode: record.openAppCode,
      appKey: `mock-rotated-open-app-key-${mockSequence += 1}`,
      appKeyFingerprint: fingerprint,
    },
  };
}

export function replacePlatformOpenAppUserGrantsMock(
  openAppCode: string,
  payload: ReplaceOpenAppUserGrantsPayload,
) {
  const record = requireRecord(openAppCode);
  record.userGrants = uniqueStrings(payload.userCodes).map((userCode) => ({ userCode }));
  record.updatedAt = nowIso();
  return cloneDetail(record);
}

export function replacePlatformOpenAppCapabilityGrantsMock(
  openAppCode: string,
  payload: ReplaceOpenAppCapabilityGrantsPayload,
) {
  const record = requireRecord(openAppCode);
  const grants = payload.grants
    .map((grant) => ({
      capabilityCode: grant.capabilityCode.trim(),
      operationCode: grant.operationCode.trim(),
    }))
    .filter((grant) => grant.capabilityCode && grant.operationCode);
  for (const grant of grants) {
    if (!isKnownGrant(grant.capabilityCode, grant.operationCode)) {
      throw new Error("capability grant is invalid");
    }
  }
  const uniqueGrantKeys = Array.from(new Set(
    grants.map((grant) => `${grant.capabilityCode}:${grant.operationCode}`),
  )).sort();
  record.capabilityGrants = uniqueGrantKeys.map((key) => {
    const [capabilityCode, operationCode] = key.split(":");
    return { capabilityCode, operationCode };
  });
  record.updatedAt = nowIso();
  return cloneDetail(record);
}
