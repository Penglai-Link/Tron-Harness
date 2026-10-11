import { requestApi } from "@/shared/api/httpClient";

export interface CapabilityPackRecord {
  id: string;
  name: string;
  version: string;
  description: string;
  provider: string;
  updatedAt: string | null;
  effective: boolean;
  kind: "capability" | "plugin";
  contents: CapabilityContentRecord[];
}

export interface CapabilityContentRecord {
  kind: string;
  name: string;
  description: string;
}

export interface ExtensionImplementationRecord {
  extensionPoint: string;
  extensionName: string;
  implementation: string;
  description: string;
  providerPackId: string;
  provider: string;
  effective: boolean;
}

export function listCapabilityPacks(signal?: AbortSignal) {
  return requestApi<CapabilityPackRecord[]>("GET /api/v1/capabilities/packs", {
    signal,
  });
}

export function listExtensionImplementations(signal?: AbortSignal) {
  return requestApi<ExtensionImplementationRecord[]>(
    "GET /api/v1/capabilities/extensions",
    { signal }
  );
}

export function setCapabilityPackEnabled(packId: string, enabled: boolean) {
  return requestApi<CapabilityPackRecord>(
    "PUT /api/v1/capabilities/packs/{id}/enabled",
    {
      pathParams: { id: packId },
      body: { enabled },
    }
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

export function importCapabilityPack(file: File) {
  const body = new FormData();
  body.append("file", file);
  return requestApi<CapabilityPackRecord>(
    "POST /api/v1/capabilities/packs/import",
    { body }
  );
}

export function uninstallCapabilityPack(id: string) {
  return requestApi<unknown>("DELETE /api/v1/capabilities/packs/{id}", {
    pathParams: { id },
  });
}
