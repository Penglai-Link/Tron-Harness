import {
  useEffect,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";

import {
  deletePlatformVariable,
  fetchPlatformVariables,
  revealPlatformVariable,
  updatePlatformVariable,
} from "@/domain/variable/api";
import {
  deletePlatformVariableRecord,
  listPlatformVariableRecords,
  revealPlatformVariableRecord,
  updatePlatformVariableRecord,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/components/ui/button";
import { Field } from "@/components/forms";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  PlatformVariableFormFields,
  ReadValue,
  SensitiveDataBadge,
  SensitiveRevealIconButton,
  SensitiveValueCode,
  emptyPlatformVariableForm,
  findPlatformVariable,
  formatPlatformVariableTimestamp,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
  normalizePlatformVariableDescription,
  platformVariableFormFromRecord,
} from "./platformVariablePageHelpers";

export function PlatformVariableDetailsPage() {
  const { t } = useTranslation(["pages"]);
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const variableKey = searchParams.get("variableKey") ?? "";
  const startsInEditMode = searchParams.get("mode") === "edit";
  const [record, setRecord] = useState<PlatformVariableRecord | null>(null);
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [revealedValue, setRevealedValue] = useState("");

  useEffect(() => {
    if (!variableKey) return;

    let active = true;

    async function loadVariable() {
      setLoading(true);
      try {
        const records = dataSource === "api"
          ? await fetchPlatformVariables()
          : listPlatformVariableRecords();
        if (active) {
          setRecord(findPlatformVariable(records, variableKey));
          setRevealedValue("");
        }
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : t("pages:variablesPlatform.errors.load", "Platform variable could not be loaded"));
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadVariable();

    return () => {
      active = false;
    };
  }, [dataSource, toast, variableKey]);

  useEffect(() => {
    if (!record) return;
    setForm(platformVariableFormFromRecord(record));
  }, [record]);

  useEffect(() => {
    if (!record) return;
    setEditing(startsInEditMode);
  }, [record, startsInEditMode]);

  if (!variableKey) {
    return <Navigate replace to="/settings/variables" />;
  }

  function detailPath(key = variableKey) {
    return `/settings/variables/detail?variableKey=${encodeURIComponent(key)}`;
  }

  function enterEditing() {
    if (!record) return;
    setForm(platformVariableFormFromRecord(record));
    setEditing(true);
    setRevealedValue("");
    navigate(`${detailPath(record.key)}&mode=edit`, { replace: true });
  }

  function exitEditing(key = variableKey) {
    setEditing(false);
    navigate(detailPath(key), { replace: true });
  }

  function resetDraft() {
    if (!record) return;
    setForm(platformVariableFormFromRecord(record));
    exitEditing(record.key);
  }

  async function saveVariable() {
    if (!record) return;
    const wasSensitive = isSensitiveRecord(record);
    if (wasSensitive && !form.isSensitive && !form.value.trim()) {
      toast.error(t("pages:variables.errors.newValueRequired", "New value is required before turning off sensitivity"));
      return;
    }
    if (form.isSensitive && !form.value.trim()) {
      toast.error(t("pages:variables.errors.valueRequired", "Variable value is required"));
      return;
    }
    if (!wasSensitive && form.isSensitive && !form.value.trim()) {
      toast.error(t("pages:variables.errors.sensitiveValueRequired", "Sensitive value is required before turning on sensitivity"));
      return;
    }

    setSubmitting(true);
    try {
      const payload = {
        description: normalizePlatformVariableDescription(form.description),
        isSensitive: form.isSensitive,
        value: form.value,
        valueMode: "REPLACE" as const,
      };
      const saved = dataSource === "api"
        ? await updatePlatformVariable(record.key, payload)
        : updatePlatformVariableRecord(record.key, payload);
      toast.success(t("pages:variables.messages.saved", "{{key}} saved", { key: saved.key }));
      setRecord(saved);
      setRevealedValue("");
      exitEditing(saved.key);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variables.errors.save", "Platform variable could not be saved"));
    } finally {
      setSubmitting(false);
    }
  }

  async function removeVariable() {
    if (!record) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deletePlatformVariable(record.key);
      } else {
        deletePlatformVariableRecord(record.key);
      }
      toast.success(t("pages:variables.messages.deleted", "{{key}} deleted", { key: record.key }));
      navigate("/settings/variables");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variablesPlatform.errors.delete", "Platform variable could not be deleted"));
    } finally {
      setDeleting(false);
    }
  }

  async function revealSensitiveValue() {
    if (!record) return;
    try {
      const revealed = dataSource === "api"
        ? await revealPlatformVariable({
          variableKey: record.key,
        })
        : revealPlatformVariableRecord(record.key);
      setRevealedValue(revealed.value);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Sensitive value could not be revealed");
    }
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:variablesPlatform.title", "Variables"), href: "/settings/variables" }, { label: t("pages:variables.loading", "Loading") }]}
          title={t("pages:variablesPlatform.detailTitle", "Platform Variable")}
        />
        <SurfaceCard>{t("pages:variablesPlatform.loading", "Loading platform variable...")}</SurfaceCard>
      </>
    );
  }

  if (!record) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:variablesPlatform.title", "Variables"), href: "/settings/variables" }, { label: t("pages:variables.notFound", "Not Found") }]}
          title={t("pages:variablesPlatform.detailTitle", "Platform Variable")}
        />
        <SurfaceCard>{t("pages:variablesPlatform.notFound", "Platform variable not found.")}</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: t("pages:variablesPlatform.title", "Variables"), href: "/settings/variables" }, { label: record.key }]}
        title={record.key}
        actions={editing ? (
          <>
            <Button variant="secondary" disabled={submitting} onClick={resetDraft}>

              {t("pages:variables.actions.cancel", "Cancel")}
            </Button>
            <Button disabled={submitting} aria-busy={submitting} onClick={() => void saveVariable()}>

              {submitting ? t("pages:variables.actions.saving", "Saving...") : t("pages:variables.actions.save", "Save")}
            </Button>
          </>
        ) : (
          <>
            <Button onClick={enterEditing}>

              {t("pages:variables.actions.edit", "Edit")}
            </Button>
            <Button variant="ghost" disabled={deleting || loading} aria-busy={deleting} onClick={() => void removeVariable()}>

              {deleting ? t("pages:variables.actions.deleting", "Deleting...") : t("pages:variables.actions.delete", "Delete")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <SurfaceCard className="space-y-6">
          {editing ? (
            <PlatformVariableFormFields form={form} keyEditable={false} onChange={setForm} />
          ) : (
            <div className="grid gap-x-8 gap-y-6 md:grid-cols-2">
              <Field label={t("pages:variables.form.key", "Variable Key")}>
                <ReadValue>
                  <code>{record.key}</code>
                </ReadValue>
              </Field>
              <Field label={t("pages:variables.form.sensitive", "Sensitive Data")}>
                <SensitiveDataBadge sensitive={isSensitiveRecord(record)} />
              </Field>
              <Field label={t("pages:variablesPlatform.scope", "Scope")}>
                <ReadValue>{t("pages:variablesPlatform.platformConfiguration", "Platform Configuration")}</ReadValue>
              </Field>
              <Field label={t("pages:variables.detail.created", "Created")}>
                <ReadValue>{formatPlatformVariableTimestamp(record.createdAt)}</ReadValue>
              </Field>
              <Field label={t("pages:variables.detail.updated", "Updated")}>
                <ReadValue>{formatPlatformVariableTimestamp(record.updatedAt)}</ReadValue>
              </Field>
              <Field label={t("pages:variablesPlatform.identifier", "Identifier")}>
                <ReadValue>{record.id}</ReadValue>
              </Field>
              <div className="md:col-span-2">
                <Field className="w-full" label={t("pages:variables.form.value", "Variable Value")}>
                  <div className="min-h-32 w-full rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <SensitiveValueCode
                        className="min-w-0 flex-1"
                        state={getSensitiveState(isSensitiveRecord(record), revealedValue)}
                        testId={`platform-sensitive-value-${record.key}`}
                      >
                        {getSensitiveDisplayValue(record, revealedValue)}
                      </SensitiveValueCode>
                      {isSensitiveRecord(record) ? (
                        <SensitiveRevealIconButton
                          revealed={Boolean(revealedValue)}
                          variableKey={record.key}
                          onClick={() => {
                            if (revealedValue) {
                              setRevealedValue("");
                            } else {
                              void revealSensitiveValue();
                            }
                          }}
                        />
                      ) : null}
                    </div>
                  </div>
                </Field>
              </div>
              <div className="md:col-span-2">
                <Field className="w-full" label={t("pages:variables.form.description", "Description")}>
                  <div className="min-h-20 w-full whitespace-pre-wrap break-words rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm leading-6 text-[var(--color-on-surface)]">
                    {record.description ?? ""}
                  </div>
                </Field>
              </div>
            </div>
          )}
        </SurfaceCard>
      </div>
    </>
  );
}
