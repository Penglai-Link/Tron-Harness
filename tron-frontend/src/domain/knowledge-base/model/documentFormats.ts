// MinerU 4 input contract, mirrored by document/preflight.rs and checked by the API.
export const knowledgeDocumentExtensions = ["pdf", "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "jp2", "doc", "docx", "ppt", "pptx", "xls", "xlsx", "rtf", "odt", "ods", "odp", "epub", "ofd", "html", "htm", "shtml", "csv", "tsv", "txt", "md", "markdown", "rst", "tex"] as const;
export const knowledgeDocumentAccept = knowledgeDocumentExtensions.map((ext) => `.${ext}`).join(",");
export const isKnowledgeDocumentSupported = (name: string) => (knowledgeDocumentExtensions as readonly string[]).includes(name.split(".").at(-1)?.toLowerCase() ?? "");
