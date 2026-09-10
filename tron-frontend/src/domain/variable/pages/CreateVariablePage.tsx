import { useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { createVariable } from "@/domain/variable/api";
import { createVariableRecord } from "@/domain/variable/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  WorkspaceVariableFormFields,
  emptyWorkspaceVariableForm,
  normalizeWorkspaceVariableDescription,
} from "./workspaceVariablePageHelpers";

export function CreateVariablePage() {
  const { t } = useTranslation(["pages"]);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [form, setForm] = useState(emptyWorkspaceVariableForm());
  const [submitting, setSubmitting] = useState(false);

  async function saveVariable() {
    if (!activeWorkspaceCode) {
      toast.error(t("pages:variables.errors.workspaceRequired", "Select a workspace before creating a variable"));
      return;
    }

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
        description: normalizeWorkspaceVariableDescription(form.description),
        isSensitive: form.isSensitive,
        valueMode: "REPLACE" as const,
      };
      const saved = dataSource === "api"
        ? await createVariable(activeWorkspaceCode, payload)
        : createVariableRecord(activeWorkspaceCode, payload);
      toast.success(t("pages:variables.messages.created", "{{key}} created", { key: saved.key }));
      navigate(`/variables/detail?variableKey=${encodeURIComponent(saved.key)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variables.errors.create", "Variable could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: t("pages:variables.title", "Variables"), href: "/variables" }, { label: t("pages:variables.actions.new", "New") }]}
        title={t("pages:variables.createTitle", "Create Variable")}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/variables")}>{t("pages:variables.actions.cancel", "Cancel")}</Button>
            <Button size="lg" disabled={submitting || !activeWorkspaceCode || !form.key.trim()} aria-busy={submitting} onClick={() => void saveVariable()}>
              {submitting ? t("pages:variables.actions.saving", "Saving...") : t("pages:variables.actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <div>
        {!activeWorkspaceCode ? (
          <SurfaceCard>{t("pages:variables.errors.workspaceRequired", "Select a workspace in the shell before creating a variable.")}</SurfaceCard>
        ) : (
          <SurfaceCard>
            <WorkspaceVariableFormFields
              form={form}
              keyEditable
              mode="create"
              onChange={setForm}
            />
          </SurfaceCard>
        )}
      </div>
    </>
  );
}
