import { useState } from "react";
import { useTranslation } from "react-i18next";

import { createPlatformVariable } from "@/domain/variable/api";
import { createPlatformVariableRecord } from "@/domain/variable/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";

import {
  PlatformVariableFormFields,
  emptyPlatformVariableForm,
  normalizePlatformVariableDescription,
} from "./platformVariablePageHelpers";

export function CreatePlatformVariableDialog({ onClose, onCreated }: {
  onClose: () => void;
  onCreated: () => void;
}) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [submitting, setSubmitting] = useState(false);

  async function saveVariable() {
    if (submitting) return;
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
      onCreated();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variablesPlatform.errors.create", "Public variable could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !submitting) onClose(); }}>
      <DialogContent className="sm:max-w-2xl" showCloseButton={!submitting}>
        <DialogHeader>
          <DialogTitle>{t("pages:variablesPlatform.createTitle", "Create Public Variable")}</DialogTitle>
        </DialogHeader>
        <form className="space-y-5" onSubmit={(event) => { event.preventDefault(); void saveVariable(); }}>
          <fieldset disabled={submitting}>
            <PlatformVariableFormFields form={form} keyEditable onChange={setForm} />
          </fieldset>
          <DialogFooter>
            <Button type="button" variant="secondary" disabled={submitting} onClick={onClose}>{t("pages:variables.actions.cancel", "Cancel")}</Button>
            <Button type="submit" disabled={submitting || !form.key.trim()} aria-busy={submitting}>
              {submitting ? t("pages:variables.actions.saving", "Saving...") : t("pages:variables.actions.save", "Save")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
