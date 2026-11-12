import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { AlertCircle, LoaderCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { MarkdownContent } from "@/components/markdown-content/MarkdownContent";
import {
  getCenterDetail,
  getCenterVersions,
  isCenterReleaseUnavailable,
  type CenterPack,
} from "../api/capabilityCenterApi";

interface Props {
  pack: CenterPack;
  busy: boolean;
  error: Error | null;
  onReviewChange: () => void;
  onClose: () => void;
  onInstall: (releaseId: string, installedVersion: string | null) => void;
}
export function CapabilityReleaseDialog({
  pack,
  busy,
  error,
  onReviewChange,
  onClose,
  onInstall,
}: Props) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`capabilityCenter.${key}`);
  const [releaseId, setReleaseId] = useState(pack.releaseId!);
  const [releaseVersion, setReleaseVersion] = useState(pack.version);
  const [versionPage, setVersionPage] = useState(1);
  const client = useQueryClient();
  const [unavailableReleases, setUnavailableReleases] = useState<Set<string>>(
    () => new Set()
  );
  const detail = useQuery({
    queryKey: ["capability-center", "detail", releaseId, i18n.resolvedLanguage],
    queryFn: ({ signal }) => getCenterDetail(releaseId, signal),
    retry: false,
    enabled: !unavailableReleases.has(releaseId),
  });
  const unavailable =
    unavailableReleases.has(releaseId) ||
    isCenterReleaseUnavailable(detail.error) ||
    isCenterReleaseUnavailable(error);
  useEffect(() => {
    if (!unavailable || unavailableReleases.has(releaseId)) return;
    setUnavailableReleases((previous) => new Set(previous).add(releaseId));
    // Refresh discover/installed projections and published history without retrying this release.
    void Promise.all([
      client.invalidateQueries({ queryKey: ["capability-center", "page"] }),
      client.invalidateQueries({
        queryKey: ["capability-center", "categories"],
      }),
      client.invalidateQueries({
        queryKey: ["capability-center", "versions", pack.packId],
      }),
    ]);
  }, [client, pack.packId, releaseId, unavailable, unavailableReleases]);
  const versions = useQuery({
    queryKey: ["capability-center", "versions", pack.packId, versionPage],
    queryFn: ({ signal }) =>
      getCenterVersions(pack.packId, versionPage, signal),
    retry: false,
  });
  const data = detail.data;
  const fileSize = data?.pack.fileSize;
  const sizeUnit =
    fileSize != null && fileSize >= 1024 * 1024
      ? "MiB"
      : fileSize != null && fileSize >= 1024
      ? "KiB"
      : "B";
  const sizeDivisor =
    sizeUnit === "MiB" ? 1024 * 1024 : sizeUnit === "KiB" ? 1024 : 1;
  const sizeLabel =
    fileSize == null
      ? "—"
      : `${new Intl.NumberFormat(i18n.language, {
          maximumFractionDigits: 1,
        }).format(fileSize / sizeDivisor)} ${sizeUnit}`;
  const options = (versions.data?.records ?? []).filter(
    (version) => !unavailableReleases.has(version.releaseId)
  );
  const selectedVersion =
    options.find((v) => v.releaseId === releaseId)?.version ??
    data?.pack.version ??
    releaseVersion;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <DialogContent className="sm:max-w-2xl" showCloseButton={!busy}>
        <div className="space-y-2 pr-7">
          <DialogTitle className="break-words text-lg">{pack.name}</DialogTitle>
          <DialogDescription className="break-all font-mono text-xs">
            {pack.packId}
          </DialogDescription>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <label className="flex items-center gap-2 text-sm">
            {text("version")}
            <SelectInput
              value={releaseId}
              disabled={busy || versions.isLoading}
              aria-label={text("version")}
              className="w-40"
              onValueChange={(value) => {
                if (value) {
                  onReviewChange();
                  setReleaseVersion(
                    options.find(
                      (version) => version.releaseId === String(value)
                    )?.version ?? releaseVersion
                  );
                  setReleaseId(String(value));
                }
              }}
            >
              {!options.some((v) => v.releaseId === releaseId) && (
                <SelectItem value={releaseId} disabled={unavailable}>
                  {selectedVersion}
                  {unavailable ? ` · ${text("unavailableVersion")}` : ""}
                </SelectItem>
              )}
              {options.map((v) => (
                <SelectItem key={v.releaseId} value={v.releaseId}>
                  {v.version}
                </SelectItem>
              ))}
            </SelectInput>
          </label>
          {(versionPage > 1 || (versions.data?.total ?? 0) > 20) && (
            <>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy || versionPage === 1 || versions.isFetching}
                onClick={() => setVersionPage((p) => p - 1)}
              >
                {text("newerVersions")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={
                  busy ||
                  !versions.data ||
                  versionPage * 20 >= versions.data.total ||
                  versions.isFetching
                }
                onClick={() => setVersionPage((p) => p + 1)}
              >
                {text("olderVersions")}
              </Button>
            </>
          )}
          <Badge variant="outline">{text(`sources.${pack.source}`)}</Badge>
        </div>
        {versions.isError && (
          <p role="alert" className="text-sm text-[var(--color-warning)]">
            {text("versionsFailed")}{" "}
            <Button variant="link" onClick={() => void versions.refetch()}>
              {text("retry")}
            </Button>
          </p>
        )}
        {unavailable ? (
          <div
            role="alert"
            className="space-y-2 rounded-md bg-[var(--color-surface-container)] p-4 text-sm"
          >
            <p className="font-medium">{text("releaseUnavailable")}</p>
            <p className="leading-6 text-[var(--color-on-surface-variant)]">
              {text("releaseUnavailableHint")}
            </p>
          </div>
        ) : detail.isLoading ? (
          <p role="status">{text("loading")}</p>
        ) : detail.isError ? (
          <div role="alert">
            <p>{detail.error.message}</p>
            <Button variant="outline" onClick={() => void detail.refetch()}>
              {text("retry")}
            </Button>
          </div>
        ) : (
          data && (
            <>
              <p className="whitespace-pre-wrap break-words text-sm leading-6 text-[var(--color-on-surface-variant)]">
                {data.pack.description}
              </p>
              <dl className="grid grid-cols-2 gap-4 border-y border-[var(--color-outline-variant)] py-4 text-sm sm:grid-cols-3">
                {[
                  ["author", data.pack.author],
                  [
                    "likes",
                    data.pack.likeCount == null
                      ? "—"
                      : new Intl.NumberFormat(i18n.language).format(
                          data.pack.likeCount
                        ),
                  ],
                  [
                    "downloads",
                    data.pack.downloads == null
                      ? "—"
                      : new Intl.NumberFormat(i18n.language).format(
                          data.pack.downloads
                        ),
                  ],
                  ["framework", data.manifest.framework],
                  ["size", sizeLabel],
                  [
                    "installedVersion",
                    data.pack.installedVersion ?? text("notInstalled"),
                  ],
                  ["signature", data.signingKeyId ?? text("unsigned")],
                ].map(([key, value]) => (
                  <div key={key}>
                    <dt className="text-xs text-[var(--color-on-surface-variant)]">
                      {text(key)}
                    </dt>
                    <dd className="mt-1 break-words">{value}</dd>
                  </div>
                ))}
              </dl>
              <div className="grid gap-4 sm:grid-cols-2">
                <section className="min-w-0">
                  <h3 className="mb-2 text-sm font-medium">
                    {text("permissions")}
                  </h3>
                  <ul className="space-y-1 break-all font-mono text-xs">
                    {data.manifest.permissions?.length ? (
                      data.manifest.permissions.map((p) => <li key={p}>{p}</li>)
                    ) : (
                      <li className="font-sans text-[var(--color-on-surface-variant)]">
                        {text("noneDeclared")}
                      </li>
                    )}
                  </ul>
                </section>
                <section className="min-w-0">
                  <h3 className="mb-2 text-sm font-medium">
                    {text("dependencies")}
                  </h3>
                  <ul className="space-y-1 break-all font-mono text-xs">
                    {data.manifest.dependencies?.length ? (
                      data.manifest.dependencies.map((d) => (
                        <li key={d.id}>
                          {d.id} {d.version}
                        </li>
                      ))
                    ) : (
                      <li className="font-sans text-[var(--color-on-surface-variant)]">
                        {text("noneDeclared")}
                      </li>
                    )}
                  </ul>
                </section>
              </div>
              {data.releaseNotes && (
                <section>
                  <h3 className="mb-2 font-medium">{text("releaseNotes")}</h3>
                  <MarkdownContent content={data.releaseNotes} />
                </section>
              )}
              {data.readme && (
                <details className="min-w-0">
                  <summary className="cursor-pointer py-2 text-sm font-medium">
                    {text("readme")}
                  </summary>
                  <MarkdownContent content={data.readme} />
                </details>
              )}
              <div className="flex items-start gap-2 rounded-md bg-[var(--color-surface-container)] p-3 text-xs leading-5 text-[var(--color-on-surface-variant)]">
                <AlertCircle
                  size={16}
                  aria-hidden="true"
                  className="mt-0.5 shrink-0"
                />
                <p>{text("installNotice")}</p>
              </div>
              {data.installBlockReason && (
                <p
                  role="status"
                  className="text-sm text-[var(--color-warning)]"
                >
                  {text("blocked")}: {data.installBlockReason}
                </p>
              )}
            </>
          )
        )}
        {error && !unavailable && (
          <p
            role="alert"
            className="break-words text-sm text-[var(--color-error)]"
          >
            {error.message}
          </p>
        )}
        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            {text(unavailable ? "backToCenter" : "close")}
          </Button>
          <Button
            data-dialog-primary
            disabled={
              busy ||
              unavailable ||
              !data?.installable ||
              detail.isFetching ||
              detail.isError
            }
            onClick={() => onInstall(releaseId, data!.pack.installedVersion)}
          >
            {busy && (
              <LoaderCircle
                size={16}
                aria-hidden="true"
                className="animate-spin motion-reduce:animate-none"
              />
            )}
            {text(
              busy
                ? "installing"
                : data?.pack.installedVersion
                ? "confirmUpdate"
                : "confirmInstall"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
