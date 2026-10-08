import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useAuthStore } from "@/store/authStore";
import { downloadResourceBinary } from "../api/resourceApi";
import type { ResourceKind } from "../model/ResourceModel";

type PreviewFormat = "pdf" | "image" | "text" | "unsupported";

export function resourcePreviewFormat(fileName: string, kind: ResourceKind, mimeType = ""): PreviewFormat {
  const extension = fileName.split(".").at(-1)?.toLowerCase();
  if (extension === "pdf") return "pdf";
  if (["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico"].includes(extension ?? "")) return "image";
  if (["txt", "md", "markdown", "json", "csv", "tsv", "log", "yaml", "yml", "xml", "html", "htm", "css", "js", "ts", "tsx", "jsx", "rs", "py", "java", "sql", "sh"].includes(extension ?? "")) return "text";
  if (mimeType === "application/pdf" || kind === "PDF") return "pdf";
  if (mimeType.startsWith("image/") || kind === "IMAGE") return "image";
  if (mimeType.startsWith("text/") || mimeType === "application/json") return "text";
  return "unsupported";
}

/** Shared by a single resource file and the knowledge document's aggregate view. */
export function ResourceFilePreview({ workspaceCode, resourceCode, fileName, kind, revision = "", compact = false, fullscreen = false, format }: {
  workspaceCode: string; resourceCode: string; fileName: string; kind: ResourceKind;
  revision?: string; compact?: boolean; fullscreen?: boolean; format?: PreviewFormat;
}) {
  const { t } = useTranslation("pages");
  const user = useAuthStore(state => state.session?.user.userCode ?? "");
  const toast = useToast();
  const resource = useQuery({
    queryKey: ["resource-file-preview", user, workspaceCode, resourceCode, revision, fileName, kind, format],
    gcTime: 0, retry: false,
    enabled: Boolean(user && workspaceCode && resourceCode),
    queryFn: async ({ signal }) => {
      const response = await downloadResourceBinary({ workspaceCode, resourceCode }, signal);
      const resolvedFormat = format ?? resourcePreviewFormat(fileName, kind, response.blob.type);
      return { blob: response.blob, format: resolvedFormat, text: resolvedFormat === "text" ? await response.blob.text() : null };
    },
  });
  const [object, setObject] = useState<{ blob: Blob; url: string } | null>(null);
  useEffect(() => {
    const data = resource.data;
    if (!data || (data.format !== "pdf" && data.format !== "image")) return;
    const blob = data.format === "pdf" ? new Blob([data.blob], { type: "application/pdf" }) : data.blob;
    const url = URL.createObjectURL(blob);
    setObject({ blob: data.blob, url });
    return () => URL.revokeObjectURL(url);
  }, [resource.data]);
  useEffect(() => {
    if (resource.error) toast.error(resource.error.message);
  }, [resource.error, toast]);
  const url = object?.blob === resource.data?.blob ? object?.url : undefined;
  if (resource.isPending) return <p role="status">{t("resourceEditor.preview.loading")}</p>;
  if (resource.error) return <Button variant="outline" onClick={() => void resource.refetch()}>{t("resourceEditor.preview.retry")}</Button>;
  if (resource.data?.format === "unsupported") return <p className="text-sm text-[var(--color-on-surface-variant)]">{t("resourceEditor.preview.unsupported")}</p>;
  if (resource.data?.format === "text") return <pre aria-label={fileName} className={`${fullscreen ? "h-full" : "max-h-[65vh]"} overflow-auto whitespace-pre-wrap break-words font-mono text-sm`}>{resource.data.text}</pre>;
  if (!url) return null;
  return resource.data?.format === "pdf"
    ? <iframe title={fileName} src={url} className={fullscreen ? "h-full w-full border-0" : compact ? "h-80 w-full rounded border-0" : "h-[65vh] min-h-80 w-full rounded border-0"} />
    : <img alt={fileName} src={url} className={fullscreen ? "h-full w-full object-contain" : compact ? "max-h-72 w-full object-contain" : "max-h-[65vh] w-full object-contain"} />;
}
