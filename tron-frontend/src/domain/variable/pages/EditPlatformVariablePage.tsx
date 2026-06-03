import {
  useEffect,
  useState,
} from "react";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Save,
  X,
} from "lucide-react";

import {
  fetchPlatformVariables,
  updatePlatformVariable,
} from "@/domain/variable/api";
import {
  listPlatformVariableRecords,
  updatePlatformVariableRecord,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
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
  findPlatformVariable,
  isSensitiveRecord,
  normalizePlatformVariableDescription,
  platformVariableFormFromRecord,
} from "./platformVariablePageHelpers";

export function EditPlatformVariablePage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const variableKey = searchParams.get("variableKey") ?? "";
  const [record, setRecord] = useState<PlatformVariableRecord | null>(null);
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!variableKey) return;

    let active = true;

    async function loadVariable() {
      setLoading(true);
      try {
        const records = dataSource === "api"
          ? await fetchPlatformVariables()
          : listPlatformVariableRecords();
        const nextRecord = findPlatformVariable(records, variableKey);
        if (!active) return;
        setRecord(nextRecord);
        if (nextRecord) setForm(platformVariableFormFromRecord(nextRecord));
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : "Platform variable could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadVariable();

    return () => {
      active = false;
    };
  }, [dataSource, toast, variableKey]);

  if (!variableKey) {
    return <Navigate replace to="/settings/variables" />;
  }

  async function saveVariable() {
    if (!record) return;
    const wasSensitive = isSensitiveRecord(record);
    if (wasSensitive && !form.isSensitive && !form.value.trim()) {
      toast.error("New value is required before turning off sensitivity");
      return;
    }
    if (!wasSensitive && form.isSensitive && !form.value.trim()) {
      toast.error("Sensitive value is required before turning on sensitivity");
      return;
    }

    setSubmitting(true);
    try {
      const payload = {
        description: normalizePlatformVariableDescription(form.description),
        isSensitive: form.isSensitive,
        valueMode: form.isSensitive ? form.valueMode : "REPLACE" as const,
        ...(form.isSensitive && form.valueMode === "PRESERVE" ? {} : { value: form.value }),
      };
      const saved = dataSource === "api"
        ? await updatePlatformVariable(record.key, payload)
        : updatePlatformVariableRecord(record.key, payload);
      toast.success(`${saved.key} saved`);
      navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(saved.key)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "Loading" }]}
          title="Edit Platform Variable"
        />
        <SurfaceCard>Loading platform variable...</SurfaceCard>
      </>
    );
  }

  if (!record) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "Not Found" }]}
          title="Edit Platform Variable"
        />
        <SurfaceCard>Platform variable not found.</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: "Variables", href: "/settings/variables" },
          { label: record.key, href: `/settings/variables/detail?variableKey=${encodeURIComponent(record.key)}` },
          { label: "Edit" },
        ]}
        title={`Edit ${record.key}`}
      />

      <div className="max-w-5xl">
        <SurfaceCard className="space-y-6">
          <PlatformVariableFormFields form={form} keyEditable={false} onChange={setForm} />
          <div className="flex justify-end gap-2 border-t border-[var(--color-outline-variant)] pt-5">
            <Button variant="secondary" disabled={submitting} onClick={() => navigate(`/settings/variables/detail?variableKey=${encodeURIComponent(record.key)}`)}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
            <Button disabled={submitting} aria-busy={submitting} onClick={() => void saveVariable()}>
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </div>
        </SurfaceCard>
      </div>
    </>
  );
}
