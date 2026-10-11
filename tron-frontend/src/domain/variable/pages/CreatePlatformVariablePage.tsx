import { useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { createPlatformVariable } from "@/domain/variable/api";
import { createPlatformVariableRecord } from "@/domain/variable/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  PlatformVariableFormFields,
  emptyPlatformVariableForm,
  normalizePlatformVariableDescription,
} from "./platformVariablePageHelpers";

export function CreatePlatformVariablePage() {
  const { t } = useTranslation(["pages"]);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [submitting, setSubmitting] = useState(false);

  async function saveVariable() {
    const key = form.key.trim();
    if (!key) {
      toast.error(t("pages:variables.errors.keyRequired", "Variable key is required"));
      return;
    }

    setSubmitting(true);
    try {
      const payload = {
        key,
        value: form.value,
        description: normalizePlatformVariableDescription(form.description),
        isSensitive: form.isSensitive,
        valueMode: "REPLACE" as const,
      };
      const saved = dataSource === "api"
        ? await createPlatformVariable(payload)
        : createPlatformVariableRecord(payload);
      toast.success(t("pages:variables.messages.created", "{{key}} created", { key: saved.key }));
      navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(saved.key)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variablesPlatform.errors.create", "Platform variable could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: t("pages:variablesPlatform.title", "Variables"), href: "/settings/variables" }, { label: t("pages:variablesPlatform.actions.new", "New") }]}
        title={t("pages:variablesPlatform.createTitle", "Create Platform Variable")}
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/variables")}>{t("pages:variables.actions.cancel", "Cancel")}</Button>
            <Button disabled={submitting || !form.key.trim()} aria-busy={submitting} onClick={() => void saveVariable()}>

              {submitting ? t("pages:variables.actions.saving", "Saving...") : t("pages:variables.actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl">
        <SurfaceCard className="space-y-6">
          <PlatformVariableFormFields form={form} keyEditable onChange={setForm} />
        </SurfaceCard>
      </div>
    </>
  );
}
