import type { ResourceRecord } from "@/domain/resource/model/model";

const resources: ResourceRecord[] = [
  { id: "res-prod-db", name: "Production DB (Primary)", type: "PostgreSQL 14", owner: "Data Platform", scope: "Workspace", size: "3.8 TB", status: "Active", location: "us-east-1a", lastAccessed: "Just now", updated: "Just now" },
  { id: "res-user-uploads", name: "User Uploads Archive", type: "AWS S3 Bucket", owner: "Platform Storage", scope: "Workspace", size: "2.4 TB", status: "Active", location: "eu-west-2", lastAccessed: "2 mins ago", updated: "2 mins ago" },
  { id: "res-auth-configs", name: "Global Auth Configs", type: "JSON Config File", owner: "Identity", scope: "Inherited", size: "18 MB", status: "Inherited", location: "Global Edge", lastAccessed: "1 hr ago", updated: "1 hr ago" },
  { id: "res-hero-assets", name: "Marketing Hero Assets", type: "Media Library", owner: "Brand", scope: "App", size: "392 GB", status: "Active", location: "CDN Core", lastAccessed: "5 hrs ago", updated: "5 hrs ago" },
  { id: "res-legacy-redis", name: "Legacy Redis Cache", type: "Redis 5.0", owner: "Infrastructure", scope: "Workspace", size: "92 GB", status: "Deprecated", location: "us-west-2b", lastAccessed: "3 days ago", updated: "3 days ago" },
];

export function listResources() {
  return resources;
}
