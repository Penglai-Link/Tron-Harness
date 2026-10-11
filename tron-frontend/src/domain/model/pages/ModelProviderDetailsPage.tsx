import { useCallback, useEffect, useState } from "react";

import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";

import { useToast } from "@/components/ui/toast";
import { PageHeader, SurfaceCard, useLocaleFormatters } from "@/shared";
import {
  createModelProviderAuthorization,
  disconnectModelProvider,
  fetchProviderAccountBalance,
  fetchModelProviderAuthorization,
  fetchModelProviderConnection,
  pollModelProviderAuthorization,
  syncModelProvider,
  type ModelProviderConnection,
  type ProviderAccountBalance,
  type ProviderAuthorizationSession,
} from "@/domain/model/api";
import { ModelProviderStatusBadge } from "@/domain/model/components/ModelProviderStatusBadge";

const personAiProviderCode = "PersonAi" as const;

function detailPath(authorizationSessionId?: string) {
  const query = new URLSearchParams({ providerCode: personAiProviderCode });
  if (authorizationSessionId)
    query.set("authorizationSessionId", authorizationSessionId);
  return `/settings/model-providers/detail?${query.toString()}`;
}

function DetailLabel({
  help,
  helpAriaLabel,
  label,
}: {
  help: string;
  helpAriaLabel: string;
  label: string;
}) {
  return (
    <dt className="flex items-center gap-1.5 text-[13px] text-[var(--color-on-surface-variant)]">
      <span>{label}</span>
      <InfoTooltip ariaLabel={helpAriaLabel}>{help}</InfoTooltip>
    </dt>
  );
}

export function ModelProviderDetailsPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const authorizationSessionId = searchParams.get("authorizationSessionId");
  const [authorization, setAuthorization] =
    useState<ProviderAuthorizationSession | null>(null);
  const [connection, setConnection] = useState<ModelProviderConnection | null>(
    null,
  );
  const [accountBalance, setAccountBalance] =
    useState<ProviderAccountBalance | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<
    "authorize" | "sync" | "disconnect" | "poll" | ""
  >("");
  const [disconnectOpen, setDisconnectOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const nextConnection = await fetchModelProviderConnection(personAiProviderCode);
      setConnection(nextConnection);
      if (
        nextConnection.status === "Connected" ||
        nextConnection.status === "Degraded"
      ) {
        try {
          setAccountBalance(
            await fetchProviderAccountBalance(personAiProviderCode),
          );
        } catch {
          setAccountBalance(null);
        }
      } else {
        setAccountBalance(null);
      }
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.loadConnectionFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, [t, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!authorizationSessionId) {
      setAuthorization(null);
      return;
    }
    let active = true;
    void fetchModelProviderAuthorization(
      personAiProviderCode,
      authorizationSessionId,
    )
      .then((session) => {
        if (active) setAuthorization(session);
      })
      .catch((error) => {
        if (active) {
          toast.error(
            error instanceof Error
              ? error.message
              : t("modelProviders.messages.loadAuthorizationFailed"),
          );
        }
      });
    return () => {
      active = false;
    };
  }, [authorizationSessionId, t, toast]);

  function clearAuthorization() {
    setAuthorization(null);
    navigate(detailPath(), { replace: true });
  }

  async function beginAuthorization() {
    setBusy("authorize");
    try {
      const session =
        await createModelProviderAuthorization(personAiProviderCode);
      setAuthorization(session);
      navigate(detailPath(session.sessionId), { replace: true });
      window.open(session.loginUrl, "_blank", "noopener,noreferrer");
      toast.info(t("modelProviders.messages.authorizationPageOpened"));
      await load();
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.openAuthorizationFailed"),
      );
    } finally {
      setBusy("");
    }
  }

  async function pollAuthorization(showPending = true) {
    const session = authorization;
    if (!session) return;
    setBusy("poll");
    try {
      const result = await pollModelProviderAuthorization(
        personAiProviderCode,
        session.sessionId,
      );
      if (result.completed) {
        clearAuthorization();
        toast.success(t("modelProviders.messages.authorizationCompleted"));
        await load();
      } else {
        setAuthorization(result.session);
        if (showPending)
          toast.info(t("modelProviders.messages.authorizationPending"));
      }
    } catch (error) {
      if (showPending) {
        toast.error(
          error instanceof Error
            ? error.message
            : t("modelProviders.messages.pollAuthorizationFailed"),
        );
      }
    } finally {
      setBusy("");
    }
  }

  useEffect(() => {
    if (!authorization?.sessionId) return;
    const timer = window.setInterval(() => {
      void pollAuthorization(false);
    }, 3_000);
    return () => window.clearInterval(timer);
  }, [authorization?.sessionId]);

  async function syncNow() {
    setBusy("sync");
    try {
      const run = await syncModelProvider(personAiProviderCode);
      toast.success(
        t("modelProviders.messages.syncCompleted", {
          count: run.modelCount ?? 0,
        }),
      );
      await load();
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.syncFailed"),
      );
      await load();
    } finally {
      setBusy("");
    }
  }

  async function disconnect() {
    setBusy("disconnect");
    try {
      await disconnectModelProvider(personAiProviderCode);
      clearAuthorization();
      setDisconnectOpen(false);
      toast.success(t("modelProviders.messages.disconnected"));
      await load();
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.disconnectFailed"),
      );
    } finally {
      setBusy("");
    }
  }

  const connectionReady =
    connection?.status === "Connected" || connection?.status === "Degraded";

  return (
    <>
      <PageHeader
        title="Person-AI"
        actions={
          <>
            <Button
              variant="secondary"
              onClick={() =>
                navigate(
                  `/settings/model-providers/catalog?providerCode=${personAiProviderCode}`,
                )
              }
            >

              {t("modelProviders.actions.remoteModels")}
            </Button>
            <Button
              variant="secondary"
              onClick={() =>
                navigate(
                  `/settings/model-providers/sync-runs?providerCode=${personAiProviderCode}`,
                )
              }
            >

              {t("modelProviders.actions.syncHistory")}
            </Button>
            <Button
              disabled={loading || Boolean(busy)}
              variant="secondary"
              onClick={() => void load()}
            >

              {t("modelProviders.actions.refresh")}
            </Button>
          </>
        }
      />

      <SurfaceCard className="space-y-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-semibold">
              {t("modelProviders.detail.connection")}
            </h2>
            <InfoTooltip ariaLabel={t("modelProviders.aria.connectionHelp")}>
              {t("modelProviders.help.connection")}
            </InfoTooltip>
            <ModelProviderStatusBadge
              status={connection?.status ?? "Disconnected"}
            />
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={Boolean(busy)}
              size="sm"
              onClick={() => void beginAuthorization()}
            >

              {connectionReady
                ? t("modelProviders.actions.reauthorize")
                : t("modelProviders.actions.authorize")}
            </Button>
            <Button
              disabled={!connectionReady || Boolean(busy)}
              size="sm"
              variant="secondary"
              onClick={() => void syncNow()}
            >

              {t("modelProviders.actions.syncNow")}
            </Button>
            <Button
              disabled={!connection || Boolean(busy)}
              size="sm"
              variant="secondary"
              onClick={() => setDisconnectOpen(true)}
            >

              {t("modelProviders.actions.disconnect")}
            </Button>
          </div>
        </div>

        {authorization ? (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-[8px] border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3">
            <div className="flex min-w-0 items-center gap-1.5 text-sm">
              <span>{t("modelProviders.detail.authorizationSession")}</span>
              <InfoTooltip
                ariaLabel={t("modelProviders.aria.authorizationSessionHelp")}
              >
                {t("modelProviders.help.authorizationSession")}
              </InfoTooltip>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="secondary"
                onClick={() =>
                  window.open(
                    authorization.loginUrl,
                    "_blank",
                    "noopener,noreferrer",
                  )
                }
              >

                {t("modelProviders.actions.openAuthorizationPage")}
              </Button>
              <Button
                disabled={Boolean(busy)}
                size="sm"
                variant="secondary"
                onClick={() => void pollAuthorization()}
              >

                {t("modelProviders.actions.checkAuthorization")}
              </Button>
            </div>
          </div>
        ) : null}

        <dl className="grid gap-4 text-sm sm:grid-cols-2 xl:grid-cols-4">
          <div>
            <DetailLabel
              help={t("modelProviders.help.serverBaseUrl")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.serverBaseUrl"),
              })}
              label={t("modelProviders.detail.serverBaseUrl")}
            />
            <dd className="mt-1 break-all font-mono text-xs">
              {connection?.serverBaseUrl ?? "-"}
            </dd>
          </div>
          <div>
            <DetailLabel
              help={t("modelProviders.help.lastSyncAt")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.lastSyncAt"),
              })}
              label={t("modelProviders.detail.lastSyncAt")}
            />
            <dd className="mt-1">
              {connection?.lastSyncAt
                ? formatDateTime(connection.lastSyncAt, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  })
                : t("modelProviders.values.neverSynced")}
            </dd>
          </div>
          <div>
            <DetailLabel
              help={t("modelProviders.help.catalogModels")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.catalogModels"),
              })}
              label={t("modelProviders.detail.catalogModels")}
            />
            <dd className="mt-1">{connection?.catalogModelCount ?? 0}</dd>
          </div>
          <div>
            <DetailLabel
              help={t("modelProviders.help.authorizationExpiresAt")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.authorizationExpiresAt"),
              })}
              label={t("modelProviders.detail.authorizationExpiresAt")}
            />
            <dd className="mt-1">
              {connection?.authorizationExpiresAt
                ? formatDateTime(connection.authorizationExpiresAt, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  })
                : t("modelProviders.values.notProvided")}
            </dd>
          </div>
        </dl>

        {connection?.lastError ? (
          <p className="rounded-[8px] bg-[var(--color-error-container)] px-3 py-2 text-sm text-[var(--color-on-error-container)]">
            {t("modelProviders.detail.lastError", {
              error: connection.lastError,
            })}
          </p>
        ) : null}
      </SurfaceCard>

      <SurfaceCard>
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">
            {t("modelProviders.detail.account")}
          </h2>
          <InfoTooltip ariaLabel={t("modelProviders.aria.accountHelp")}>
            {t("modelProviders.help.account")}
          </InfoTooltip>
        </div>
        <dl className="mt-5 grid gap-4 text-sm sm:grid-cols-3">
          <div>
            <DetailLabel
              help={t("modelProviders.help.username")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.username"),
              })}
              label={t("modelProviders.detail.username")}
            />
            <dd className="mt-1">
              {accountBalance?.username?.trim() || t("modelProviders.values.notProvided")}
            </dd>
          </div>
          <div>
            <DetailLabel
              help={t("modelProviders.help.vipLevel")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.vipLevel"),
              })}
              label={t("modelProviders.detail.vipLevel")}
            />
            <dd className="mt-1">
              {accountBalance?.vipLevel === null || accountBalance?.vipLevel === undefined
                ? t("modelProviders.values.notProvided")
                : t("modelProviders.values.vipLevel", {
                    level: formatNumber(accountBalance.vipLevel),
                  })}
            </dd>
          </div>
          <div>
            <DetailLabel
              help={t("modelProviders.help.remainingPoints")}
              helpAriaLabel={t("modelProviders.aria.fieldHelp", {
                label: t("modelProviders.detail.remainingPoints"),
              })}
              label={t("modelProviders.detail.remainingPoints")}
            />
            <dd className="mt-1">
              {accountBalance?.remainingPoints === null || accountBalance?.remainingPoints === undefined
                ? t("modelProviders.values.notProvided")
                : formatNumber(accountBalance.remainingPoints)}
            </dd>
          </div>
        </dl>
      </SurfaceCard>

      <ConfirmDialog
        cancelText={t("common:dialogSelect.cancel")}
        confirmText={t("modelProviders.disconnectDialog.confirm")}
        description={t("modelProviders.disconnectDialog.description")}
        open={disconnectOpen}
        pending={busy === "disconnect"}
        title={t("modelProviders.disconnectDialog.title")}
        onCancel={() => setDisconnectOpen(false)}
        onConfirm={() => {
          void disconnect();
        }}
      />
    </>
  );
}
