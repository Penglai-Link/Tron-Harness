import { requestApi } from "@/shared/api/httpClient";

export interface ToolPolicyConfig {
  smart: boolean;
}

export function getToolPolicyConfig(signal?: AbortSignal) {
  return requestApi<ToolPolicyConfig>("GET /api/v1/capabilities/extensions/tool-policy/config", { signal });
}

export function updateToolPolicyConfig(smart: boolean) {
  return requestApi<ToolPolicyConfig>("PUT /api/v1/capabilities/extensions/tool-policy/config", { body: { smart } });
}

export interface CapabilityPackRecord {
  id: string;
  name: string;
  version: string;
  cuaDriverSdkVersion: string | null;
  description: string;
  readme: string | null;
  provider: string;
  updatedAt: string | null;
  effective: boolean;
  kind: "capability" | "plugin";
  contents: CapabilityContentRecord[];
}

export interface CapabilityContentRecord {
  displayName?: string;
  kind: string;
  name: string;
  description: string;
  provides?: string[];
}

export interface ExtensionImplementationRecord {
  extensionPoint: string;
  extensionName: string;
  implementation: string;
  description: string;
  providerPackId: string;
  provider: string;
  effective: boolean;
  enabled: boolean;
  configuration: "Required" | "Extension" | "CapabilityPack";
}

export function listCapabilityPacks(signal?: AbortSignal) {
  return requestApi<CapabilityPackRecord[]>("GET /api/v1/capabilities/packs", {
    signal,
    queryParams: { includeContents: false },
  });
}

export async function getCapabilityPluginCounts(signal?: AbortSignal) {
  const packs = await requestApi<CapabilityPackRecord[]>("GET /api/v1/capabilities/packs", {
    signal,
    queryParams: { includeContents: true },
  });
  const plugins = new Set<string>();
  const enabled = new Set<string>();
  for (const pack of packs) {
    for (const item of pack.contents) {
      if (item.kind !== "Plugin") continue;
      plugins.add(item.name);
      if (pack.effective) enabled.add(item.name);
    }
  }
  return { total: plugins.size, enabled: enabled.size };
}

export async function getCapabilityPack(id: string, signal?: AbortSignal) {
  const records = await requestApi<CapabilityPackRecord[]>("GET /api/v1/capabilities/packs", {
    signal, queryParams: { id, includeContents: true },
  });
  return records.find(record => record.id === id);
}

export function listExtensionImplementations(signal?: AbortSignal) {
  return requestApi<ExtensionImplementationRecord[]>(
    "GET /api/v1/capabilities/extensions",
    { signal }
  );
}

export function setExtensionEnabled(extensionPoint: string, enabled: boolean) {
  return requestApi<ExtensionImplementationRecord[]>(
    "PUT /api/v1/capabilities/extensions/{point}/enabled",
    {
      pathParams: { point: extensionPoint },
      body: { enabled },
    }
  );
}

export async function importCapabilityPack(file: File) {
  const body = new FormData();
  body.append("file", file);
  const result = await requestApi<CapabilityPackRecord>(
    "POST /api/v1/capabilities/packs/import",
    { body }
  );
  window.dispatchEvent(new Event("tron:navigation-updated"));
  return result;
}
