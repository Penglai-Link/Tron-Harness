import type { ResourceKind, ResourceSourceKind } from "@/domain/resource/model";

export const resourceFileKinds: ResourceKind[] = ["FILE", "IMAGE", "PDF"];

export const resourceSourceFilters: (ResourceSourceKind | "ALL")[] = ["ALL", "LOCAL", "INHERITED", "OVERRIDE"];
