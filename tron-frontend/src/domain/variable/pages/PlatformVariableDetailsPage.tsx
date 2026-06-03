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
  Edit3,
  Trash2,
} from "lucide-react";

import {
  deletePlatformVariable,
  fetchPlatformVariables,
  revealPlatformVariable,
  verifyStepUpPassword,
} from "@/domain/variable/api";
import {
  deletePlatformVariableRecord,
  listPlatformVariableRecords,
  revealPlatformVariableRecord,
  verifyStepUpPasswordRecord,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import {
  ReadValue,
  SensitiveRevealAction,
  SensitiveValueCode,
  StepUpRevealDialog,
  findPlatformVariable,
  formatPlatformVariableTimestamp,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
} from "./platformVariablePageHelpers";

export function PlatformVariableDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const variableKey = searchParams.get("variableKey") ?? "";
  const [record, setRecord] = useState<PlatformVariableRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [revealedValue, setRevealedValue] = useState("");
  const [revealDialogOpen, setRevealDialogOpen] = useState(false);
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");

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
          setRevealDialogOpen(false);
          setRevealError("");
        }
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

  async function removeVariable() {
    if (!record) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deletePlatformVariable(record.key);
      } else {
        deletePlatformVariableRecord(record.key);
      }
      toast.success(`${record.key} deleted`);
      navigate("/settings/variables");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be deleted");
    } finally {
      setDeleting(false);
    }
  }

  async function revealSensitiveValue(password: string) {
    if (!record) return;
    setRevealPending(true);
    setRevealError("");
    try {
      const stepUp = dataSource === "api"
        ? await verifyStepUpPassword({
          password,
          target: {
            scope: "platform",
            variableKey: record.key,
          },
        })
        : verifyStepUpPasswordRecord({
          password,
          target: {
            scope: "platform",
            variableKey: record.key,
          },
        });
      const revealed = dataSource === "api"
        ? await revealPlatformVariable({
          variableKey: record.key,
          stepUpToken: stepUp.stepUpToken,
        })
        : revealPlatformVariableRecord(record.key, stepUp.stepUpToken);
      setRevealedValue(revealed.value);
      setRevealDialogOpen(false);
      setRevealError("");
    } catch (error) {
      setRevealError(error instanceof Error ? error.message : "Sensitive value could not be revealed");
    } finally {
      setRevealPending(false);
    }
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "Loading" }]}
          title="Platform Variable"
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
          title="Platform Variable"
        />
        <SurfaceCard>Platform variable not found.</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: record.key }]}
        title={record.key}
        actions={(
          <>
            <Button onClick={() => navigate(`/settings/variables/edit?variableKey=${encodeURIComponent(record.key)}`)}>
              <Edit3 size={16} aria-hidden="true" />
              Edit
            </Button>
            <Button variant="ghost" disabled={deleting} aria-busy={deleting} onClick={() => void removeVariable()}>
              <Trash2 size={16} aria-hidden="true" />
              {deleting ? "Deleting..." : "Delete"}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <SurfaceCard className="space-y-6">
          <div className="grid gap-4 border-b border-[var(--color-outline-variant)] pb-5 md:grid-cols-3">
            <Field label="Scope">
              <ReadValue>Platform Configuration</ReadValue>
            </Field>
            <Field label="Created">
              <ReadValue>{formatPlatformVariableTimestamp(record.createdAt)}</ReadValue>
            </Field>
            <Field label="Updated">
              <ReadValue>{formatPlatformVariableTimestamp(record.updatedAt)}</ReadValue>
            </Field>
          </div>

          <div className="grid gap-x-8 gap-y-6 md:grid-cols-2">
            <Field label="Variable Key">
              <ReadValue>
                <code>{record.key}</code>
              </ReadValue>
            </Field>
            <div className="md:col-span-2">
              <Field label="Variable Value">
                <div className="min-h-32 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <SensitiveValueCode
                      className="min-w-0 flex-1"
                      state={getSensitiveState(isSensitiveRecord(record), revealedValue)}
                      testId={`platform-sensitive-value-${record.key}`}
                    >
                      {getSensitiveDisplayValue(record, revealedValue)}
                    </SensitiveValueCode>
                    <SensitiveRevealAction
                      isSensitive={isSensitiveRecord(record)}
                      onHide={() => setRevealedValue("")}
                      onReveal={() => {
                        setRevealError("");
                        setRevealDialogOpen(true);
                      }}
                      revealed={Boolean(revealedValue)}
                      variableKey={record.key}
                    />
                  </div>
                </div>
              </Field>
            </div>
            <div className="md:col-span-2">
              <Field label="Description">
                <div className="min-h-20 rounded-lg border border-[var(--color-outline-variant)] p-4 text-sm leading-6 text-[var(--color-on-surface)]">
                  {record.description || "None"}
                </div>
              </Field>
            </div>
          </div>
        </SurfaceCard>
      </div>
      <StepUpRevealDialog
        error={revealError}
        onCancel={() => {
          setRevealDialogOpen(false);
          setRevealError("");
        }}
        onSubmit={(password) => void revealSensitiveValue(password)}
        open={revealDialogOpen}
        pending={revealPending}
        variableKey={record.key}
      />
    </>
  );
}
