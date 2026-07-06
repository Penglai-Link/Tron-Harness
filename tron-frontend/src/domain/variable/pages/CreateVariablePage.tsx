import { useState } from "react";
import { useNavigate } from "react-router";

import { createVariable } from "@/domain/variable/api";
import { createVariableRecord } from "@/domain/variable/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  WorkspaceVariableFormFields,
  emptyWorkspaceVariableForm,
  normalizeWorkspaceVariableDescription,
} from "./workspaceVariablePageHelpers";

export function CreateVariablePage() {
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
      toast.error("Select a workspace before creating a variable");
      return;
    }

    const key = form.key.trim();
    if (!key) {
      toast.error("Variable key is required");
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
      toast.success(`${saved.key} created`);
      navigate(`/variables/detail?variableKey=${encodeURIComponent(saved.key)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Variable could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Variables", href: "/variables" }, { label: "New" }]}
        title="Create Variable"
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/variables")}>Cancel</Button>
            <Button size="lg" disabled={submitting || !activeWorkspaceCode || !form.key.trim()} aria-busy={submitting} onClick={() => void saveVariable()}>
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        )}
      />

      <div>
        {!activeWorkspaceCode ? (
          <SurfaceCard>Select a workspace in the shell before creating a variable.</SurfaceCard>
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
