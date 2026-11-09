import type { ResourceKind, ResourceSourceKind } from "@/domain/resource/model";

export type ResourceSourceCode = "Local" | "Inherited" | "Override";
export type ResourceKindCode = "File" | "Image" | "Pdf";

const resourceKindCodes: Record<ResourceKind, ResourceKindCode> = {
  FILE: "File",
  IMAGE: "Image",
  PDF: "Pdf",
};

const resourceSourceCodes: Record<ResourceSourceKind, ResourceSourceCode> = {
  LOCAL: "Local",
  INHERITED: "Inherited",
  OVERRIDE: "Override",
};

export function toResourceKindCode(value: ResourceKind): ResourceKindCode {
  return resourceKindCodes[value];
}

export function fromResourceKindCode(value: ResourceKindCode): ResourceKind {
  switch (value) {
    case "File": return "FILE";
    case "Image": return "IMAGE";
    case "Pdf": return "PDF";
  }
}

export function toResourceSourceCode(value: ResourceSourceKind): ResourceSourceCode {
  return resourceSourceCodes[value];
}

export function fromResourceSourceCode(value: ResourceSourceCode): ResourceSourceKind {
  switch (value) {
    case "Local": return "LOCAL";
    case "Inherited": return "INHERITED";
    case "Override": return "OVERRIDE";
  }
}

export interface ResourceApiPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface ResourceApiFileRecord {
  fileName: string;
  fileExtension: string;
  byteSize: number;
}

export interface ResourceApiRecord {
  workspaceCode: string;
  sourceKind: ResourceSourceCode;
  resourceCode: string;
  kind: ResourceKindCode;
  name: string;
  description: string;
  allowInheritance: boolean;
  file: ResourceApiFileRecord | null;
  attributes: Record<string, unknown> | null;
  createdAt: number;
  updatedAt: number;
}

export interface ResourcePageApiRequest {
  workspaceCode: string;
  kinds: ResourceKindCode[];
  sourceKind: ResourceSourceCode | null;
  search: string | null;
  current: number;
  size: number;
}

export interface ResourceGetApiRequest {
  workspaceCode: string;
  resourceCode: string;
}

export interface ResourceCreateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  kind: "FILE" | "IMAGE" | "PDF";
  name: string;
  description: string;
  allowInheritance: boolean;
  file: File;
}

export interface ResourceUpdateApiRequest {
  workspaceCode: string;
  resourceCode: string;
  name?: string | null;
  description?: string | null;
  allowInheritance?: boolean | null;
}

export interface ResourceCommitApiResult {
  resourceCode: string;
  kind: ResourceKindCode;
}

export interface ResourceDeleteApiRequest {
  workspaceCode: string;
  resourceCode: string;
}

export interface ResourceDeleteApiResult {
  resourceCode: string;
  deleted: boolean;
}

export interface ResourceBinaryUploadApiRequest {
  workspaceCode: string;
  resourceCode: string;
  file: File;
}

export interface ResourceBinaryUploadApiResult {
  action: "UPLOADED";
  resourceCode: string;
  fileName: string;
  byteSize: number;
  targetPath: string | null;
}

export interface ResourceBinaryDownloadApiRequest {
  workspaceCode: string;
  resourceCode: string;
}
