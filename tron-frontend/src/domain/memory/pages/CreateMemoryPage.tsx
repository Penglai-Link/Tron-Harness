import {
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router";
import { Save } from "lucide-react";

import { useAuthSession } from "@/domain/auth";
import {
  MemoryFormFields,
  emptyMemoryFormState,
  memoryFormToInput,
  validateMemoryForm,
} from "@/domain/memory/components";
import {
  createMemoryRecord,
  useMemoryTargets,
} from "@/domain/memory";
import type { MemorySearchTarget } from "@/domain/memory/model";
import { createMemoryFragment } from "@/domain/memory/mock";
import {
  PageHeader,
  SurfaceCard,
  useConsoleShellConfig,
  useToast,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { Button } from "@/components/ui/button"

function buildDisplayTargets(
  targets: MemorySearchTarget[],
  workspaceLabel?: string,
  workspaceCode?: string,
  userLabel?: string,
  userCode?: string,
) {
  return targets.map((target) => {
    if (target.id === "space:current") {
      return {
        ...target,
        code: workspaceCode || target.code,
        label: workspaceLabel || target.label,
      };
    }

    if (target.id === "user:current") {
      return {
        ...target,
        code: userCode || target.code,
        label: userLabel || target.label,
      };
    }

    return target;
  });
}

export function CreateMemoryPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("memory");
  const { session } = useAuthSession();
  const {
    selectedPrimarySpace,
    selectedPrimarySpaceId,
    selectedSecondarySpace,
    selectedSecondarySpaceId,
  } = useWorkspaceSpaces();
  const workspaceLabel = selectedSecondarySpace?.name ?? selectedPrimarySpace?.name;
  const workspaceCode = selectedSecondarySpaceId || selectedPrimarySpaceId;
  const userLabel = session?.user.username || session?.user.displayName;
  const userCode = session?.user.userCode;
  const rawTargets = useMemoryTargets(workspaceCode);
  const safeTargets = Array.isArray(rawTargets) ? rawTargets : [];
  const targets = useMemo(
    () => buildDisplayTargets(safeTargets, workspaceLabel, workspaceCode, userLabel, userCode),
    [safeTargets, userCode, userLabel, workspaceCode, workspaceLabel],
  );
  const [form, setForm] = useState(() => emptyMemoryFormState());
  const [submitting, setSubmitting] = useState(false);
  const fallbackTarget = useMemo<MemorySearchTarget>(() => ({
    code: workspaceCode || "space:current",
    description: "Current space",
    id: "space:current",
    label: workspaceLabel || "Current space",
    resultCount: 0,
    scope: "space",
  }), [workspaceCode, workspaceLabel]);
  const targetOptions = targets.length ? targets : [fallbackTarget];
  const domainTargets = targetOptions.filter((target) => target.scope === "domain");
  const selectedTarget = targetOptions.find((target) => target.id === form.targetId) ?? fallbackTarget;

  useConsoleShellConfig({ searchPlaceholder: "Search memories..." });

  async function saveMemory() {
    const validationError = validateMemoryForm(form, selectedTarget);
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSubmitting(true);
    try {
      const payload = memoryFormToInput(form, selectedTarget);
      const saved = dataSource === "api" && workspaceCode
        ? await createMemoryRecord(workspaceCode, payload)
        : createMemoryFragment(payload);
      toast.success(`${saved.title} created`);
      navigate(`/memory/details?memoryId=${encodeURIComponent(saved.id)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Memory", href: "/memory" }, { label: "New" }]}
        title="New Memory"
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/memory")}>Cancel</Button>
            <Button size="lg" disabled={submitting || !form.title.trim() || !form.text.trim()} aria-busy={submitting} onClick={saveMemory}>
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        )}
      />

      <div>
        <SurfaceCard>
          <MemoryFormFields
            domainTargets={domainTargets}
            mode="create"
            state={form}
            targetOptions={targetOptions}
            onChange={setForm}
          />
        </SurfaceCard>
      </div>
    </>
  );
}
