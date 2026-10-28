import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { PageHeader, SurfaceCard } from "@/shared";
import {
  createConfiguredProvider, deleteConfiguredProvider, fetchConfiguredProvider, updateConfiguredProvider,
} from "@/domain/model/api/configuredProviderApi";
import { ConfiguredProviderFields } from "@/domain/model/components/ConfiguredProviderFields";
import { ConfiguredProviderModels } from "@/domain/model/components/ConfiguredProviderModels";
import { ProviderPresetPicker } from "@/domain/model/components/ProviderPresetPicker";
import {
  configuredProviderToDraft, emptyProviderDraft, presetModelToDraft, presetToDraft,
  providerDraftToPayload, validateProviderDraft, type ConfiguredProviderDraft,
} from "@/domain/model/components/configuredProviderForm";

export function CreateConfiguredProviderPage() {
  const [params] = useSearchParams();
  return <ConfiguredProviderPage key={params.get("source") ?? "custom"} fromCatalog={params.get("source") === "catalog"} />;
}

export function ConfiguredProviderPage({ providerId, fromCatalog = false }: { providerId?: string; fromCatalog?: boolean }) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const navigate = useNavigate();
  const toast = useToast();
  const [draft, setDraft] = useState<ConfiguredProviderDraft>(emptyProviderDraft);
  const [loading, setLoading] = useState(Boolean(providerId));
  const [loadError, setLoadError] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    if (!providerId) return;
    const controller = new AbortController();
    setLoading(true);
    setLoadError("");
    fetchConfiguredProvider(providerId, controller.signal).then((record) => {
      if (!controller.signal.aborted) setDraft(configuredProviderToDraft(record));
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setLoadError(reason instanceof Error ? reason.message : "configuredProviders.errors.load");
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [providerId, revision]);

  async function save() {
    const validation = validateProviderDraft(draft);
    if (validation) { setError(text(`validation.${validation}`)); return; }
    setError("");
    setBusy(true);
    try {
      const payload = providerDraftToPayload(draft);
      const { providerId: _id, ...update } = payload;
      const record = providerId ? await updateConfiguredProvider(providerId, update) : await createConfiguredProvider(payload);
      setDraft(configuredProviderToDraft(record));
      toast.success(text("saved"));
      navigate(`/settings/model-providers/detail?providerCode=${encodeURIComponent(record.providerId)}`, { replace: true });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : text("errors.save"));
    } finally { setBusy(false); }
  }

  async function remove() {
    if (!providerId) return;
    setBusy(true);
    setError("");
    try {
      await deleteConfiguredProvider(providerId);
      toast.success(text("deleted"));
      navigate("/settings/model-providers", { replace: true });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : text("errors.delete"));
    } finally { setBusy(false); setDeleteOpen(false); }
  }

  const title = providerId ? text("editTitle") : text(fromCatalog ? "catalogTitle" : "createTitle");
  return <>
    <PageHeader title={title}
      breadcrumbs={[{ label: t("modelProviders.titles.providers"), href: "/settings/model-providers" }, { label: title }]}
      actions={<>
        <Button variant="secondary" disabled={busy} onClick={() => navigate("/settings/model-providers")}>{text("actions.back")}</Button>
        {providerId && <Button variant="destructive" disabled={busy || loading || Boolean(loadError)} onClick={() => setDeleteOpen(true)}>{text("actions.delete")}</Button>}
        <Button disabled={busy || loading || Boolean(loadError)} aria-busy={busy} onClick={() => void save()}>{text(busy ? "actions.saving" : "actions.save")}</Button>
      </>} />
    <div className="max-w-5xl">
      {loading ? <p className="inline-flex items-center gap-2" role="status"><Spinner size={16} />{text("loading")}</p>
        : loadError ? <div role="alert" className="space-y-3"><p>{loadError === "configuredProviders.errors.load" ? t(loadError) : loadError}</p><Button variant="secondary" onClick={() => setRevision((value) => value + 1)}>{text("actions.retry")}</Button></div>
          : <SurfaceCard>
            <form className="space-y-6" onSubmit={(event) => { event.preventDefault(); if (!busy) void save(); }}>
              {fromCatalog && !providerId && <ProviderPresetPicker disabled={busy} selectedModelIds={draft.models.map((model) => model.modelId)}
                onSelect={(preset) => { const next = presetToDraft(preset); if (next) { setDraft(next); setError(""); } }}
                onAddModel={(model) => setDraft((current) => current.models.some((entry) => entry.modelId === model.modelId) ? current
                  : { ...current, models: [...current.models, presetModelToDraft(model, current.protocol)] })} />}
              <ConfiguredProviderFields draft={draft} editing={Boolean(providerId)} disabled={busy} onChange={setDraft} />
              <ConfiguredProviderModels models={draft.models} protocol={draft.protocol} disabled={busy}
                onChange={(models) => setDraft((current) => ({ ...current, models }))} />
              {error && <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p>}
              <div className="flex justify-end"><Button type="submit" disabled={busy}>{text(busy ? "actions.saving" : "actions.save")}</Button></div>
            </form>
          </SurfaceCard>}
    </div>
    <ConfirmDialog open={deleteOpen} title={text("deleteTitle")} description={text("deleteDescription")}
      cancelText={text("actions.cancel")} confirmText={text("actions.delete")} pendingText={text("actions.deleting")}
      pending={busy} onCancel={() => setDeleteOpen(false)} onConfirm={() => void remove()} />
  </>;
}
