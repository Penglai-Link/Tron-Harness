import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import { ToggleSwitch } from "@/components/forms";
import {
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  createWorkspaceModel,
  fetchWorkspaceModelCatalog,
  verifyWorkspaceModelDraft,
} from "@/domain/model/api";
import {
  ModelFormFields,
  emptyModelFormState,
  modelFormToPayload,
  type ModelFormState,
} from "@/domain/model/components";
import {
  createWorkspaceModelRecord,
  getWorkspaceModelCatalog,
  verifyWorkspaceModelCreateDraftRecord,
} from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  formatWorkspaceDisplayLabel,
  formatWorkspaceModelError,
  isEditableWorkspaceLocalRecord,
} from "./workspaceModelPageHelpers";

export function CreateWorkspaceModelPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`modelRegistry.${key}`, { defaultValue, ...options })
  );
  const dataSource = getServiceDataSource("model");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceLabel = formatWorkspaceDisplayLabel(activeWorkspace, activeWorkspaceCode);
  const [records, setRecords] = useState<ModelRecord[]>([]);
  const [form, setForm] = useState<ModelFormState>(emptyModelFormState());
  const [allowInheritance, setAllowInheritance] = useState(true);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const apiKeyInputRef = useRef<HTMLInputElement | null>(null);

  const duplicateDisplayName = form.displayName.trim()
    ? records.some((record) => (
      isEditableWorkspaceLocalRecord(record, activeWorkspaceCode)
      && record.displayName === form.displayName.trim()
    ))
    : false;

  useEffect(() => {
    if (!activeWorkspaceCode) {
      setRecords([]);
      return;
    }

    let active = true;

    async function loadWorkspaceModels() {
      setLoading(true);
      try {
        const nextCatalog = dataSource === "api"
          ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
          : getWorkspaceModelCatalog(activeWorkspaceCode);
        if (active) setRecords(nextCatalog.records);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace models could not be loaded"));
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadWorkspaceModels();

    return () => {
      active = false;
    };
  }, [activeWorkspaceCode, dataSource, toast]);

  async function saveModel(options?: { verifyAfterCreate?: boolean }) {
    if (!activeWorkspaceCode) {
      toast.error(pageT("errors.workspaceRequired", "Select a workspace before creating a model"));
      return;
    }
    if (duplicateDisplayName) {
      toast.error(pageT("errors.duplicateDisplayName", "displayName already exists in workspace {{workspace}}", { workspace: activeWorkspaceLabel }));
      return;
    }

    const apiKeyRef = apiKeyInputRef.current?.value.trim() ?? "";
    if (!apiKeyRef) {
      toast.error(pageT("errors.apiKeyRequired", "apiKey is required"));
      return;
    }

    const verifyAfterCreate = Boolean(options?.verifyAfterCreate);
    setSubmitting(!verifyAfterCreate);
    setVerifying(verifyAfterCreate);
    try {
      const payload = modelFormToPayload(form, apiKeyRef, {
        enabled: true,
        allowInheritance,
      });
      if (verifyAfterCreate) {
        const verified = dataSource === "api"
          ? await verifyWorkspaceModelDraft(activeWorkspaceCode, payload)
          : verifyWorkspaceModelCreateDraftRecord(activeWorkspaceCode, payload);
        toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.displayName }));
        return;
      }

      const saved = dataSource === "api"
        ? await createWorkspaceModel(activeWorkspaceCode, payload)
        : createWorkspaceModelRecord(activeWorkspaceCode, payload);
      if (!verifyAfterCreate) {
        toast.success(pageT("messages.created", "{{name}} created", { name: saved.displayName }));
      }
      navigate(`/models/detail?modelCode=${encodeURIComponent(saved.modelCode)}`);
    } catch (error) {
      toast.error(formatWorkspaceModelError(
        error,
        activeWorkspaceLabel,
        verifyAfterCreate ? pageT("errors.verify", "Verify failed") : pageT("errors.create", "Workspace model could not be created"),
      ));
    } finally {
      setSubmitting(false);
      setVerifying(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("title", "Models"), href: "/models" }, { label: pageT("actions.new", "New") }]}
        title={pageT("detail.createWorkspaceModel", "Create Workspace Model")}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/models")}>{pageT("actions.cancel", "Cancel")}</Button>
            <Button
              size="lg" variant="outline"
              disabled={!activeWorkspaceCode || loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={verifying}
              onClick={() => void saveModel({ verifyAfterCreate: true })}
            >
              {verifying ? pageT("actions.verifying", "Verifying...") : pageT("actions.verify", "Verify")}
            </Button>
            <Button
              size="lg"
              disabled={!activeWorkspaceCode || loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={submitting}
              onClick={() => void saveModel()}
            >
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <div>
        {!activeWorkspaceCode ? (
          <SurfaceCard>{pageT("errors.workspaceRequired", "Select a workspace in the shell before creating a model.")}</SurfaceCard>
        ) : (
          <SurfaceCard className="space-y-5">
            <ModelFormFields
              state={form}
              onChange={setForm}
              apiKeyInputRef={apiKeyInputRef}
              syncModelNameFromDisplayName
              displayNameError={duplicateDisplayName ? pageT("errors.duplicateDisplayName", "displayName already exists in workspace {{workspace}}", { workspace: activeWorkspaceLabel }) : undefined}
            />

            <ToggleSwitch
              checked={allowInheritance}
              label={pageT("form.allowInheritance", "Allow Inheritance")}
              onCheckedChange={setAllowInheritance}
            />
          </SurfaceCard>
        )}
      </div>
    </>
  );
}
