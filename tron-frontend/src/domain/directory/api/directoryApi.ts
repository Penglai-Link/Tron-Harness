import { requestApi } from "@/shared/api/httpClient";

export interface DirectoryRecord {
  directoryCode: string;
  workspaceCode: string;
  executionHost: string;
  path: string;
  name: string;
  parentDirectoryCode: string | null;
  available: boolean;
}

export interface DirectoryBrowserPage {
  path: string;
  parent: string | null;
  entries: Array<{ name: string; path: string }>;
}

export function browseDirectories(workspaceCode: string, path: string | null, signal?: AbortSignal): Promise<DirectoryBrowserPage> {
  return requestApi("GET /api/v1/directories/browse", { queryParams: { workspaceCode, path }, signal });
}

export function listDirectories(workspaceCode: string, signal?: AbortSignal): Promise<DirectoryRecord[]> {
  return requestApi("GET /api/v1/directories?workspaceCode={workspaceCode}", {
    pathParams: { workspaceCode }, signal,
  });
}

export function registerDirectory(workspaceCode: string, path: string): Promise<DirectoryRecord> {
  return requestApi("POST /api/v1/directories", { body: { workspaceCode, path } });
}

export function renameDirectory(directoryCode: string, name: string): Promise<void> {
  return requestApi("PATCH /api/v1/directories/{directoryCode}", {
    pathParams: { directoryCode }, body: { name },
  });
}

export function archiveDirectory(directoryCode: string): Promise<{ sessionCodes: string[] }> {
  return requestApi("POST /api/v1/directories/{directoryCode}/archive", { pathParams: { directoryCode } });
}

export function pickDirectory(workspaceCode: string): Promise<DirectoryRecord | null> {
  return requestApi("POST /api/v1/directories/pick", {
    body: { workspaceCode }, headers: { "X-Tron-Native-Picker": "1" }, retryOnUnauthorized: false,
  });
}

export function pickDirectoryPath(workspaceCode: string): Promise<{ path: string | null }> {
  return requestApi("POST /api/v1/directories/pick-path", {
    body: { workspaceCode }, headers: { "X-Tron-Native-Picker": "1" }, retryOnUnauthorized: false,
  });
}

export function deleteDirectory(directoryCode: string): Promise<{ sessionCodes: string[] }> {
  return requestApi("DELETE /api/v1/directories/{directoryCode}", { pathParams: { directoryCode } });
}
