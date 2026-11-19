import { Package } from "lucide-react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { CapabilityLikeButton } from "./CapabilityLikeButton";
import { capabilityDescription } from "../model/capabilityDescription";
import type { CenterPack } from "../api/capabilityCenterApi";

interface Props {
  records: CenterPack[];
  busy: boolean;
  queuedPackIds?: Set<string>;
  onDetail: (pack: CenterPack) => void;
  onToggle: (pack: CenterPack) => void;
  onRemove: (pack: CenterPack) => void;
}
export function CapabilityCenterList({
  records,
  busy,
  queuedPackIds,
  onDetail,
  onToggle,
  onRemove,
}: Props) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`capabilityCenter.${key}`);
  return (
    <ul
      className="grid min-w-0 grid-cols-[repeat(auto-fill,minmax(min(100%,18rem),1fr))] gap-4"
      aria-label={text("results")}
    >
      {records.map((pack) => (
        <li key={pack.packId} className="min-w-0">
          <Card
            data-settings-card
            data-theme-part="capability-center-card"
            className="relative isolate h-full min-w-0 gap-3 p-4"
          >
            {/* A stretched native link/button is a sibling of the actions, never their parent. */}
            <Button
              variant="ghost"
              className="absolute inset-0 z-0 h-full w-full rounded-[inherit] p-0"
              aria-label={t("capabilityCenter.viewPack", { name: pack.name })}
              disabled={busy}
              nativeButton={!pack.installedVersion}
              render={
                pack.installedVersion ? (
                  <Link
                    to={`/settings/capability-packs/${encodeURIComponent(
                      pack.packId
                    )}`}
                  />
                ) : undefined
              }
              onClick={pack.installedVersion ? undefined : () => onDetail(pack)}
            />
            <div className="pointer-events-none relative flex min-w-0 items-start gap-3">
              <Package
                size={21}
                aria-hidden="true"
                className="mt-0.5 shrink-0 text-[var(--color-on-surface-variant)]"
              />
              <div className="min-w-0 flex-1 space-y-2">
                <h2 className="line-clamp-2 break-words text-sm font-semibold leading-5">
                  {pack.name}
                </h2>
                <div className="flex flex-wrap items-center gap-1.5">
                  <Badge variant="outline">
                    {text(`sources.${pack.source}`)}
                  </Badge>
                  {pack.updateAvailable ? (
                    <Badge variant="warning">{text("updatable")}</Badge>
                  ) : pack.installedVersion ? (
                    <Badge variant={pack.enabled ? "success" : "outline"}>
                      {text(pack.enabled ? "enabled" : "disabled")}
                    </Badge>
                  ) : null}
                  {queuedPackIds?.has(pack.packId) && (
                    <Badge variant="info">{text("queue.queued")}</Badge>
                  )}
                  {pack.category && (
                    <span className="break-words text-xs text-[var(--color-on-surface-variant)]">
                      {pack.category}
                    </span>
                  )}
                </div>
              </div>
            </div>
            <div className="pointer-events-none relative min-w-0 flex-1 space-y-3">
              <p className="line-clamp-2 min-h-12 break-words text-sm leading-6 text-[var(--color-on-surface-variant)]">
                {pack.builtin
                  ? capabilityDescription(
                      t,
                      "Pack",
                      pack.packId,
                      pack.description
                    )
                  : pack.releaseId
                  ? pack.description
                  : text("localDescription")}
              </p>
              <p className="truncate font-mono text-xs text-[var(--color-on-surface-variant)]">
                {pack.packId}
              </p>
              <div className="flex min-w-0 flex-wrap items-center justify-between gap-x-3 gap-y-1 text-xs text-[var(--color-on-surface-variant)]">
                <span className="min-w-0 break-words">{pack.author}</span>
                <span className="break-all font-mono tabular-nums">
                  {pack.updateAvailable
                    ? `${pack.installedVersion} → ${pack.version}`
                    : pack.installedVersion ?? pack.version}
                </span>
              </div>
              {pack.updateCheckFailed && (
                <p className="text-xs text-[var(--color-warning)]">
                  {text("updateUnknown")}
                </p>
              )}
            </div>
            <div className="pointer-events-none relative mt-auto flex flex-wrap items-center justify-between gap-2 border-t border-[var(--color-outline-variant)] pt-3">
              {pack.likeCount != null ? (
                <CapabilityLikeButton pack={pack} disabled={busy} />
              ) : (
                <span />
              )}
              <div className="flex flex-wrap items-center justify-end gap-2">
                {pack.installedVersion && (
                  <>
                    {!pack.builtin && (
                      <Button
                        className="pointer-events-auto border-[var(--color-error)]/40 text-[var(--color-error)]"
                        variant="outline"
                        size="sm"
                        disabled={busy || queuedPackIds?.has(pack.packId)}
                        onClick={() => onRemove(pack)}
                      >
                        {text("uninstall")}
                      </Button>
                    )}
                    <Button
                      className="pointer-events-auto"
                      variant="outline"
                      size="sm"
                      disabled={busy || queuedPackIds?.has(pack.packId)}
                      onClick={() => onToggle(pack)}
                    >
                      {text(pack.enabled ? "disable" : "enable")}
                    </Button>
                  </>
                )}
                {pack.releaseId &&
                  (!pack.installedVersion || pack.updateAvailable) && (
                    <Button
                      className="pointer-events-auto"
                      size="sm"
                      disabled={busy}
                      onClick={() => onDetail(pack)}
                    >
                      {text(pack.updateAvailable ? "update" : "install")}
                    </Button>
                  )}
              </div>
            </div>
          </Card>
        </li>
      ))}
    </ul>
  );
}
