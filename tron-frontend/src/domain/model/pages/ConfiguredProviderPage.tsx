import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard } from "@/shared";
import { ProviderPageHeader } from "@/domain/model/components/ProviderPageHeader";
import {
  createConfiguredProvider, deleteConfiguredProvider, fetchConfiguredProvider, fetchProviderPresets, updateConfiguredProvider, type ProviderPreset,
} from "@/domain/model/api/configuredProviderApi";
import { ConfiguredProviderFields } from "@/domain/model/components/ConfiguredProviderFields";
import { ConfiguredProviderModels } from "@/domain/model/components/ConfiguredProviderModels";
import { ProviderPresetPicker } from "@/domain/model/components/ProviderPresetPicker";
import {
  configuredProviderToDraft, emptyProviderDraft, presetToDraft,
  providerDraftToPayload, validateProviderDraft, type ConfiguredProviderDraft,
} from "@/domain/model/components/configuredProviderForm";
import type { ReasoningEffort } from "@/domain/model/model";

export function CreateConfiguredProviderPage() {
  return <ConfiguredProviderPage />;
}

export function ConfiguredProviderPage({ providerId }: { providerId?: string }) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const navigate = useNavigate();
  const toast = useToast();
  const [draft, setDraft] = useState<ConfiguredProviderDraft>(emptyProviderDraft);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [presetId, setPresetId] = useState<string | null>(null);
  const [modelSectionRevision, setModelSectionRevision] = useState(0);
  const [providerSelected, setProviderSelected] = useState(false);
  const [loading, setLoading] = useState(Boolean(providerId));
  const [loadError, setLoadError] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    if (!providerId) return;
    const controller = new AbortController();
    fetchProviderPresets(controller.signal).then(setPresets).catch(() => {});
    return () => controller.abort();
  }, [providerId]);

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

  function changeDefaultReasoningEfforts(defaultReasoningEfforts: ReasoningEffort[]) {
    setDraft((current) => ({
      ...current,
      defaultReasoningEfforts,
      models: current.models.map((model) => (
        model.reasoning
          && model.modelType === "Text"
          && !model.supportedReasoningEfforts.length
          && model.defaultReasoningEffort
          && !defaultReasoningEfforts.includes(model.defaultReasoningEffort)
          ? { ...model, defaultReasoningEffort: null }
          : model
      )),
    }));
  }

  async function save() {
    const validation = validateProviderDraft(draft);
    if (validation) { setError(text(`validation.${validation}`)); return; }
    setError("");
    setBusy(true);
    try {
      const payload = providerDraftToPayload(draft);
      const record = providerId ? await updateConfiguredProvider(providerId, payload) : await createConfiguredProvider(payload);
      setDraft(configuredProviderToDraft(record));
      toast.success(text("saved"));
      navigate("/settings/model-providers", { replace: true });
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

  const typeName = draft.providerType === "custom" ? text("custom") : presets.find((preset) => preset.id === draft.providerType)?.name ?? (draft.providerType || text("providerTypeUnknown"));
  const title = providerId ? text("editTitle") : text("createTitle");
  return <>
    <ProviderPageHeader title={title}
      actions={<>
        <Button variant="secondary" disabled={busy} onClick={() => navigate("/settings/model-providers")}>{text("actions.back")}</Button>
        {providerId && <Button variant="destructive" disabled={busy || loading || Boolean(loadError)} onClick={() => setDeleteOpen(true)}>{text("actions.delete")}</Button>}
        <Button disabled={busy || loading || Boolean(loadError) || (!providerId && !providerSelected)} aria-busy={busy} onClick={() => void save()}>{text(busy ? "actions.saving" : "actions.save")}</Button>
      </>} />
    <div className="max-w-5xl">
      {loading ? <p className="inline-flex items-center gap-2" role="status"><Spinner size={16} />{text("loading")}</p>
        : loadError ? <div role="alert" className="space-y-3"><p>{loadError === "configuredProviders.errors.load" ? t(loadError) : loadError}</p><Button variant="secondary" onClick={() => setRevision((value) => value + 1)}>{text("actions.retry")}</Button></div>
          : <SurfaceCard>
            <form className="space-y-6" onSubmit={(event) => { event.preventDefault(); if (!busy) void save(); }}>
              {!providerId && <ProviderPresetPicker disabled={busy}
                onSelectionPending={() => { setProviderSelected(false); setPresetId(null); setError(""); }}
                onSelect={(preset) => { const next = preset ? presetToDraft(preset) : emptyProviderDraft(); if (next) { setDraft(next); setModelSectionRevision(value => value + 1); setPresetId(preset?.id ?? null); setProviderSelected(true); setError(""); } }}
                onCreateFromExisting={(next) => { setDraft(next); setModelSectionRevision(value => value + 1); setPresetId(null); setProviderSelected(true); setError(""); }} />}
              {(providerId || providerSelected) && <>
              {providerId && <div className="space-y-1 text-sm"><p className="font-medium">{text("providerType")}</p><p>{typeName}</p>{!draft.providerType && <select aria-label={text("providerType")} value="" onChange={(event) => setDraft((current) => ({ ...current, providerType: event.target.value || null }))} className="w-full rounded border bg-transparent p-2"><option value="">{text("providerTypeUnknown")}</option><option value="custom">{text("custom")}</option>{presets.filter((preset) => preset.supported).map((preset) => <option key={preset.id} value={preset.id}>{preset.name}</option>)}</select>}</div>}
              <ConfiguredProviderFields draft={draft} disabled={busy} onChange={setDraft} />
              <ConfiguredProviderModels key={modelSectionRevision} presetId={presetId} models={draft.models} protocol={draft.protocol} disabled={busy}
                defaultReasoningEfforts={draft.defaultReasoningEfforts}
                onDefaultReasoningEffortsChange={changeDefaultReasoningEfforts}
                onChange={(models) => setDraft((current) => ({ ...current, models }))} />
              </>}
              {error && <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p>}
              <div className="flex justify-end"><Button type="submit" disabled={busy || (!providerId && !providerSelected)}>{text(busy ? "actions.saving" : "actions.save")}</Button></div>
            </form>
          </SurfaceCard>}
    </div>
    <ConfirmDialog open={deleteOpen} title={text("deleteTitle")} description={text("deleteDescription")}
      cancelText={text("actions.cancel")} confirmText={text("actions.delete")} pendingText={text("actions.deleting")}
      pending={busy} onCancel={() => setDeleteOpen(false)} onConfirm={() => void remove()} />
  </>;
}
