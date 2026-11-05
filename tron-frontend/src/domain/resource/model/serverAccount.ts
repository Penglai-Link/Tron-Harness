import type { ResourceRecord } from "./ResourceModel";

export interface ServerAccountBinding {
  workspaceCode: string;
  serverCode: string;
}

export function serverAccountBindings(record: ResourceRecord): ServerAccountBinding[] {
  const servers = record.attributes?.servers;
  if (!Array.isArray(servers)) return [];
  return servers.filter((server): server is ServerAccountBinding => (
    !!server && typeof server === "object"
    && typeof server.workspaceCode === "string" && typeof server.serverCode === "string"
  ));
}
