import { useState } from "react";
import { useNavigate } from "react-router";
import { Save } from "lucide-react";

import { createPlatformVariable } from "@/domain/variable/api";
import { createPlatformVariableRecord } from "@/domain/variable/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import {
  PlatformVariableFormFields,
  emptyPlatformVariableForm,
  normalizePlatformVariableDescription,
} from "./platformVariablePageHelpers";

export function CreatePlatformVariablePage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [submitting, setSubmitting] = useState(false);

  async function saveVariable() {
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
        description: normalizePlatformVariableDescription(form.description),
        isSensitive: form.isSensitive,
        valueMode: "REPLACE" as const,
      };
      const saved = dataSource === "api"
        ? await createPlatformVariable(payload)
        : createPlatformVariableRecord(payload);
      toast.success(`${saved.key} created`);
      navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(saved.key)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "New" }]}
        title="Create Platform Variable"
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/variables")}>Cancel</Button>
            <Button disabled={submitting || !form.key.trim()} aria-busy={submitting} onClick={() => void saveVariable()}>
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
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
