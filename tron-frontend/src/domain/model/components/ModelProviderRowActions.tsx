import { MoreHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import type { ModelProviderConnection } from "@/domain/model/api";

export function ModelProviderRowActions({ provider, busy, onToggle, onDelete, onAuthorize, onSync, onDisconnect }: {
  provider: ModelProviderConnection;
  busy: boolean;
  onToggle: () => void;
  onDelete: () => void;
  onAuthorize: () => void;
  onSync: () => void;
  onDisconnect: () => void;
}) {
  const { t } = useTranslation("pages");
  const connected = provider.status === "Connected";
  return <div className="flex justify-end" onClick={event => event.stopPropagation()} onKeyDown={event => event.stopPropagation()}>
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button size="icon" variant="ghost" disabled={busy}
        aria-label={t("configuredProviders.actionsFor", { name: provider.displayName })}>
        <MoreHorizontal size={16} aria-hidden="true" />
      </Button>} />
      <DropdownMenuContent align="end">
        {provider.source === "Custom" ? <>
          <DropdownMenuItem disabled={busy} onClick={onToggle}>{t(connected ? "configuredProviders.actions.disable" : "configuredProviders.actions.enable")}</DropdownMenuItem>
          <DropdownMenuItem disabled={busy} variant="destructive" onClick={onDelete}>{t("configuredProviders.actions.delete")}</DropdownMenuItem>
        </> : <>
          <DropdownMenuItem disabled={busy} onClick={onAuthorize}>{t(provider.status === "Disconnected" ? "modelProviders.actions.authorize" : "modelProviders.actions.reauthorize")}</DropdownMenuItem>
          <DropdownMenuItem disabled={busy || !["Connected", "Degraded"].includes(provider.status)} onClick={onSync}>{t("modelProviders.actions.syncNow")}</DropdownMenuItem>
          <DropdownMenuItem disabled={busy || provider.status === "Disconnected"} variant="destructive" onClick={onDisconnect}>{t("modelProviders.actions.disconnect")}</DropdownMenuItem>
        </>}
      </DropdownMenuContent>
    </DropdownMenu>
  </div>;
}
