import { useRef, useState } from "react";

import { Button, SurfaceCard } from "@/shared";

import { TargetPageFrame, type TargetPageFrameHandle, type TargetPageFrameInspectionResult } from "../components/TargetPageFrame";

const DEFAULT_TARGET_URL = "https://example.com";

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function readString(payload: Record<string, unknown> | null, key: string): string {
  const value = payload?.[key];

  return typeof value === "string" ? value : "";
}

function formatRect(payload: Record<string, unknown> | null): string {
  const rect = asRecord(payload?.rect);
  if (!rect) {
    return "-";
  }

  const x = Number(rect.x ?? 0);
  const y = Number(rect.y ?? 0);
  const width = Number(rect.width ?? 0);
  const height = Number(rect.height ?? 0);

  return `x:${x} y:${y} w:${width} h:${height}`;
}

function formatAttributes(payload: Record<string, unknown> | null): string {
  const attributes = asRecord(payload?.attributes);
  if (!attributes || Object.keys(attributes).length === 0) {
    return "-";
  }

  return Object.entries(attributes)
    .map(([key, value]) => `${key}="${String(value)}"`)
    .join(" ");
}

function formatBasicInfo(payload: Record<string, unknown> | null): string {
  const basicInfo = asRecord(payload?.basicInfo);
  if (!basicInfo || Object.keys(basicInfo).length === 0) {
    return "-";
  }

  return Object.entries(basicInfo)
    .filter(([, value]) => typeof value === "string" && value.trim().length > 0)
    .map(([key, value]) => `${key}: ${String(value)}`)
    .join("; ") || "-";
}

interface AnnotationImagePreview {
  dataUrl: string;
  height: number;
  width: number;
}

function readAnnotationImage(payload: Record<string, unknown> | null): AnnotationImagePreview | null {
  const annotationImage = asRecord(payload?.annotationImage);
  const dataUrl = annotationImage?.dataUrl;
  const width = Number(annotationImage?.width ?? 0);
  const height = Number(annotationImage?.height ?? 0);

  if (typeof dataUrl !== "string" || !dataUrl.startsWith("data:image/")) {
    return null;
  }

  return {
    dataUrl,
    height: height > 0 ? height : 1,
    width: width > 0 ? width : 1,
  };
}

export function TargetPagePage() {
  const frameRef = useRef<TargetPageFrameHandle | null>(null);
  const [latestInspection, setLatestInspection] = useState<TargetPageFrameInspectionResult | null>(null);

  const handleLoadTarget = () => {
    frameRef.current?.open(DEFAULT_TARGET_URL);
  };

  const latestInspectionPayload = latestInspection?.dom ?? null;
  const latestRemark = readString(latestInspectionPayload, "remark");
  const annotationImage = readAnnotationImage(latestInspectionPayload);

  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex w-full max-w-[1200px] flex-col gap-4">
        <div className="flex items-center justify-end">
          <Button onClick={handleLoadTarget}>Open target</Button>
        </div>
        <TargetPageFrame ref={frameRef} onInspectionConfirm={setLatestInspection} />
        {latestInspection ? (
          <SurfaceCard aria-label="Inspection result" className="space-y-4">
            <div className="flex flex-col gap-1">
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Inspection result</h2>
              <p className="text-sm text-[var(--color-on-surface-variant)]">
                {latestRemark || "No remark"}
              </p>
            </div>
            {annotationImage ? (
              <figure className="overflow-hidden rounded-md border border-[var(--color-outline-variant)] bg-white">
                <div
                  aria-label="Annotated DOM selection"
                  className="mx-auto block max-h-[360px] w-full bg-contain bg-center bg-no-repeat"
                  role="img"
                  style={{
                    aspectRatio: `${annotationImage.width} / ${annotationImage.height}`,
                    backgroundImage: `url("${annotationImage.dataUrl}")`,
                  }}
                />
              </figure>
            ) : null}
            <dl className="grid gap-3 text-sm md:grid-cols-2">
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Selector</dt>
                <dd className="mt-1 break-all font-mono text-[var(--color-on-surface)]">{readString(latestInspectionPayload, "selector") || "-"}</dd>
              </div>
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Tag</dt>
                <dd className="mt-1 font-mono text-[var(--color-on-surface)]">{readString(latestInspectionPayload, "tagName") || "-"}</dd>
              </div>
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Text</dt>
                <dd className="mt-1 break-words text-[var(--color-on-surface)]">{readString(latestInspectionPayload, "text") || "-"}</dd>
              </div>
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Rect</dt>
                <dd className="mt-1 font-mono text-[var(--color-on-surface)]">{formatRect(latestInspectionPayload)}</dd>
              </div>
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Page</dt>
                <dd className="mt-1 break-all text-[var(--color-on-surface)]">{readString(latestInspectionPayload, "title") || readString(latestInspectionPayload, "url") || "-"}</dd>
              </div>
              <div>
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Attributes</dt>
                <dd className="mt-1 break-all font-mono text-[var(--color-on-surface)]">{formatAttributes(latestInspectionPayload)}</dd>
              </div>
              <div className="md:col-span-2">
                <dt className="text-xs font-medium uppercase tracking-normal text-[var(--color-on-surface-variant)]">Basic Info</dt>
                <dd className="mt-1 break-all font-mono text-[var(--color-on-surface)]">{formatBasicInfo(latestInspectionPayload)}</dd>
              </div>
            </dl>
          </SurfaceCard>
        ) : null}
        <section className="sr-only">
          <p>Inspection selected: {latestInspection ? "yes" : "no"}</p>
        </section>
      </section>
    </main>
  );
}
