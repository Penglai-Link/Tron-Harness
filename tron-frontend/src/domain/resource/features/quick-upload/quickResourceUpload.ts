export const quickResourceTypes = ["FILE", "IMAGE", "PDF"] as const;

export type QuickResourceType = typeof quickResourceTypes[number];

export type QuickUploadBlockedReason = "FILE_TOO_LARGE";

export type QuickUploadEditorReason = "BINARY_NAME";

export const quickResourceMaxFileSizeBytes = 1_073_741_824;

export interface QuickResourceUploadInput {
  binaryName: string;
  file: Pick<File, "name" | "size">;
  pageCount?: number | null;
  type: QuickResourceType;
}

export type QuickResourceUploadDecision =
  | {
    binaryName: string;
    kind: "READY";
    type: QuickResourceType;
  }
  | {
    binaryName: string;
    kind: "EDITOR";
    reasons: QuickUploadEditorReason[];
    type: QuickResourceType;
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

export function evaluateQuickResourceUpload({ binaryName, file, type }: QuickResourceUploadInput): QuickResourceUploadDecision {
  if (file.size > quickResourceMaxFileSizeBytes) {
    return { kind: "BLOCKED", reason: "FILE_TOO_LARGE" };
  }

  const editorReasons: QuickUploadEditorReason[] = [];
  if (!binaryName.trim()) editorReasons.push("BINARY_NAME");
  if (editorReasons.length) {
    return { binaryName, kind: "EDITOR", reasons: Array.from(new Set(editorReasons)), type };
  }

  return { binaryName, kind: "READY", type };
}
