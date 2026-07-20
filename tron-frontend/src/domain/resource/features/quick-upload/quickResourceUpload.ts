import type {
  ResourceDefinitionBinarySchema,
  ResourceDefinitionEffectiveBinaryPolicy,
  ResourceDefinitionVersionDetail,
} from "@/domain/resource/model";
import { validateJsonValue } from "@/shared/ui/json-schema-draft7";

export const quickResourceTypes = ["FILE", "IMAGE", "PDF"] as const;

export type QuickResourceType = typeof quickResourceTypes[number];

export type QuickUploadBlockedReason =
  | "BINARY_UNSUPPORTED"
  | "DEFINITION_INVALID"
  | "DEFINITION_UNAVAILABLE"
  | "FILE_TOO_LARGE";

export type QuickUploadEditorReason =
  | "BINARY_NAME"
  | "PDF_PAGE_COUNT"
  | "REQUIRES_MORE_FILES"
  | "SCHEMA";

export interface QuickResourceAttributesInput {
  binaryName: string;
  file: Pick<File, "name" | "size">;
  pageCount?: number | null;
  type: QuickResourceType;
}

export interface QuickResourceUploadInput extends QuickResourceAttributesInput {
  definition: ResourceDefinitionVersionDetail | null;
}

export type QuickResourceUploadDecision =
  | {
    attributes: Record<string, unknown>;
    binaryName: string;
    definition: ResourceDefinitionVersionDetail;
    kind: "READY";
  }
  | {
    attributes: Record<string, unknown>;
    binaryName: string;
    definition: ResourceDefinitionVersionDetail;
    kind: "EDITOR";
    reasons: QuickUploadEditorReason[];
    validationErrors: string[];
  }
  | {
    kind: "BLOCKED";
    reason: QuickUploadBlockedReason;
  };

const imageFormats: Record<string, string> = {
  avif: "AVIF",
  bmp: "BMP",
  gif: "GIF",
  jpeg: "JPEG",
  jpg: "JPEG",
  png: "PNG",
  svg: "SVG",
  webp: "WEBP",
};

export function getFileExtension(fileName: string) {
  const baseName = fileName.trim().split(/[\\/]/).pop() ?? "";
  const index = baseName.lastIndexOf(".");
  if (index <= 0 || index === baseName.length - 1) return "";
  return baseName.slice(index + 1).toLowerCase();
}

export function getFileStem(fileName: string) {
  const baseName = fileName.trim().split(/[\\/]/).pop() ?? "";
  const index = baseName.lastIndexOf(".");
  return (index > 0 ? baseName.slice(0, index) : baseName).trim();
}

export function suggestQuickResourceType(fileName: string): QuickResourceType {
  const extension = getFileExtension(fileName);
  if (extension === "pdf") return "PDF";
  return imageFormats[extension] ? "IMAGE" : "FILE";
}

export function imageFormatForFileName(fileName: string) {
  return imageFormats[getFileExtension(fileName)] ?? "";
}

export function generateQuickResourceCode(fileName: string, now = Date.now()) {
  const normalized = getFileStem(fileName)
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^\x00-\x7F]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^[._-]+|[._-]+$/g, "")
    .slice(0, 64);

  return normalized || `FILE_${now}`;
}

export function buildQuickResourceAttributes({ binaryName, file, pageCount, type }: QuickResourceAttributesInput) {
  const fileExtension = getFileExtension(file.name);
  if (type === "IMAGE") {
    return {
      images: [{
        binaryName,
        fileExtension,
        fileName: file.name,
        imageFormat: imageFormatForFileName(file.name),
        sizeBytes: file.size,
      }],
    };
  }
  if (type === "PDF") {
    return {
      fileName: file.name,
      pageCount: pageCount ?? 0,
      sizeBytes: file.size,
    };
  }
  return {
    fileExtension,
    fileName: file.name,
    sizeBytes: file.size,
  };
}

type QuickBinaryPolicy = Omit<ResourceDefinitionEffectiveBinaryPolicy, "maxSizeBytes"> & {
  maxSizeBytes: number | null;
};

function effectiveQuickBinaryPolicy(
  definition: ResourceDefinitionVersionDetail,
): QuickBinaryPolicy {
  if (definition.effectiveBinaryPolicy) return definition.effectiveBinaryPolicy;

  const schema: ResourceDefinitionBinarySchema = definition.binarySchema;
  return {
    maxCount: schema.maxCount,
    maxNameLength: schema.maxNameLength,
    // Older backends did not expose their resolved default. The adapter maps a
    // missing raw value to zero, which cannot be distinguished from an
    // explicit zero-byte cap here. Do not reject a real file on that ambiguous
    // value; the server still performs the authoritative validation.
    maxSizeBytes: schema.maxSizeBytes > 0 ? schema.maxSizeBytes : null,
    minCount: schema.minCount,
    namePattern: schema.namePattern || null,
  };
}

function checkBinaryPolicy(
  policy: QuickBinaryPolicy,
  file: Pick<File, "size">,
  binaryName: string,
) {
  if (policy.maxCount < 1) return { kind: "BLOCKED" as const, reason: "BINARY_UNSUPPORTED" as const };
  if (policy.minCount > policy.maxCount || policy.maxNameLength < 1) {
    return { kind: "BLOCKED" as const, reason: "DEFINITION_INVALID" as const };
  }
  if (policy.maxSizeBytes !== null && file.size > policy.maxSizeBytes) {
    return { kind: "BLOCKED" as const, reason: "FILE_TOO_LARGE" as const };
  }
  if (policy.minCount > 1) return { kind: "EDITOR" as const, reason: "REQUIRES_MORE_FILES" as const };
  if (!binaryName || binaryName.length > policy.maxNameLength) {
    return { kind: "EDITOR" as const, reason: "BINARY_NAME" as const };
  }
  try {
    if (policy.namePattern && !new RegExp(policy.namePattern).test(binaryName)) {
      return { kind: "EDITOR" as const, reason: "BINARY_NAME" as const };
    }
  } catch {
    return { kind: "BLOCKED" as const, reason: "DEFINITION_INVALID" as const };
  }
  return { kind: "READY" as const };
}

export function evaluateQuickResourceUpload({ binaryName, definition, file, pageCount, type }: QuickResourceUploadInput): QuickResourceUploadDecision {
  if (!definition || !definition.enabled || definition.lifecycleStatus !== "ACTIVE") {
    return { kind: "BLOCKED", reason: "DEFINITION_UNAVAILABLE" };
  }

  const policy = checkBinaryPolicy(effectiveQuickBinaryPolicy(definition), file, binaryName);
  if (policy.kind === "BLOCKED") return policy;

  const attributes = buildQuickResourceAttributes({ binaryName, file, pageCount, type });
  const editorReasons: QuickUploadEditorReason[] = policy.kind === "EDITOR" ? [policy.reason] : [];
  if (type === "PDF" && (!Number.isInteger(pageCount) || (pageCount ?? 0) < 1)) {
    editorReasons.push("PDF_PAGE_COUNT");
  }
  const validationErrors = validateJsonValue(definition.attributesSchema, attributes);
  if (validationErrors.length) editorReasons.push("SCHEMA");

  if (editorReasons.length) {
    return {
      attributes,
      binaryName,
      definition,
      kind: "EDITOR",
      reasons: Array.from(new Set(editorReasons)),
      validationErrors,
    };
  }

  return { attributes, binaryName, definition, kind: "READY" };
}
