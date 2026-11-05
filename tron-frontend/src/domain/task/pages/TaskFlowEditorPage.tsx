import { Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import { useNavigate, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Field, TextInput } from "@/components/forms";
import { useToast } from "@/components/ui/toast";
import { createTaskFlow, fetchTaskFlow, updateTaskFlow } from "@/domain/task/api";
import { createMockTaskFlow, getMockTaskFlow, updateMockTaskFlow } from "@/domain/task/mock/data";
import {
  taskFlowErrorMessage,
  taskFlowStatusDisplayName,
  type TaskFlowPayload,
  type TaskFlowTranslator,
} from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { PageHeader, SurfaceCard } from "@/shared";

type EditableStatus = { name: string; required: boolean; statusCode: string };

function requiredStatuses(translate: TaskFlowTranslator): EditableStatus[] {
  return [
    { statusCode: "NOT_STARTED", name: translate("statuses.NOT_STARTED", "Not Started"), required: true },
    { statusCode: "CLOSED", name: translate("statuses.CLOSED", "Closed"), required: true },
  ];
}

export function TaskFlowEditorPage() {
  const { t } = useTranslation(["pages"]);
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const existingFlowCode = searchParams.get("flowCode") ?? "";
  const editing = Boolean(existingFlowCode);
  const flowT = (key: string, defaultValue: string) =>
    t(`pages:tasks.taskFlows.${key}`, defaultValue);
  const [flowCode, setFlowCode] = useState(existingFlowCode);
  const [name, setName] = useState("");
  const [statuses, setStatuses] = useState<EditableStatus[]>(() => requiredStatuses(flowT));
  const [loading, setLoading] = useState(editing);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!editing) return;
    let active = true;
    const load = async () => {
      setLoading(true);
      try {
        const flow = dataSource === "api" ? await fetchTaskFlow(existingFlowCode) : getMockTaskFlow(existingFlowCode);
        if (!active) return;
        setFlowCode(flow.flowCode);
        setName(flow.name);
        setStatuses(flow.statuses.map((status) => ({ statusCode: status.statusCode, name: status.name, required: status.required })));
      } catch (error) {
        if (active) toast.error(taskFlowErrorMessage(error, flowT, "errors.load", "Task flows could not be loaded"));
      } finally {
        if (active) setLoading(false);
      }
    };
    void load();
    return () => { active = false; };
  }, [dataSource, editing, existingFlowCode, toast]);

  function updateStatus(index: number, key: "name" | "statusCode", value: string) {
    setStatuses((current) => current.map((status, currentIndex) => currentIndex === index ? { ...status, [key]: value } : status));
  }

  async function save() {
    const payload: TaskFlowPayload = {
      flowCode: flowCode.trim().toUpperCase(),
      name: name.trim(),
      statuses: statuses.map((status) => ({ statusCode: status.statusCode.trim().toUpperCase(), name: status.name.trim() })),
    };
    setSaving(true);
    try {
      const flow = editing
        ? (dataSource === "api" ? await updateTaskFlow(existingFlowCode, payload) : updateMockTaskFlow(existingFlowCode, payload))
        : (dataSource === "api" ? await createTaskFlow(payload) : createMockTaskFlow(payload));
      toast.success(flowT("messages.saved", "Task flow saved"));
      navigate(`/settings/task-flows/edit?flowCode=${encodeURIComponent(flow.flowCode)}`, { replace: true });
    } catch (error) {
      toast.error(taskFlowErrorMessage(error, flowT, "errors.save", "Task flow could not be saved"));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: flowT("title", "Task Flows"), href: "/settings/task-flows" }]}
        title={editing ? flowT("editor.editTitle", "Edit task flow") : flowT("editor.createTitle", "Create task flow")}
        actions={<Button disabled={loading || saving} onClick={() => void save()}>{flowT("actions.save", "Save")}</Button>}
      />
      <SurfaceCard className="w-full max-w-4xl space-y-8 p-6 shadow-none">
        <div className="grid gap-5 sm:grid-cols-2">
          <Field required label={flowT("editor.code", "Code")}>
            <TextInput disabled={editing || loading} value={flowCode} onChange={(event) => setFlowCode(event.target.value)} />
          </Field>
          <Field required label={flowT("editor.name", "Name")}>
            <TextInput disabled={loading} value={name} onChange={(event) => setName(event.target.value)} />
          </Field>
        </div>
        <section aria-labelledby="task-flow-statuses" className="space-y-4">
          <div className="flex items-center justify-between gap-4">
            <h2 id="task-flow-statuses" className="text-sm font-semibold">{flowT("editor.statuses", "Statuses")}</h2>
            <Button size="sm" variant="outline" disabled={loading || saving} onClick={() => setStatuses((current) => [...current.slice(0, -1), { statusCode: "", name: "", required: false }, current[current.length - 1]])}>
              {flowT("actions.addStatus", "Add")}
            </Button>
          </div>
          <div className="space-y-3">
            <div aria-hidden="true" className="hidden grid-cols-[1.5rem_minmax(0,1fr)_minmax(0,1fr)_2rem] items-center gap-3 text-xs text-muted-foreground sm:grid">
              <span />
              <span>{flowT("editor.statusCode", "Status code")}</span>
              <span>{flowT("editor.statusName", "Status name")}</span>
            </div>
            {statuses.map((status, index) => (
              <div className="grid grid-cols-[1.5rem_minmax(0,1fr)_2rem] items-center gap-3 sm:grid-cols-[1.5rem_minmax(0,1fr)_minmax(0,1fr)_2rem]" key={index}>
                <span className="text-center text-xs tabular-nums text-muted-foreground">{index + 1}</span>
                <TextInput aria-label={flowT("editor.statusCode", "Status code")} disabled={status.required || loading} placeholder={flowT("editor.statusCode", "Status code")} value={status.statusCode} onChange={(event) => updateStatus(index, "statusCode", event.target.value)} />
                <TextInput className="col-start-2 row-start-2 sm:col-start-auto sm:row-start-auto" aria-label={flowT("editor.statusName", "Status name")} disabled={status.required || loading} placeholder={flowT("editor.statusName", "Status name")} value={status.required ? taskFlowStatusDisplayName(status, flowT) : status.name} onChange={(event) => updateStatus(index, "name", event.target.value)} />
                <Button className="col-start-3 row-start-1 sm:col-start-auto sm:row-start-auto" aria-label={flowT("aria.deleteStatus", "Delete status")} disabled={status.required || loading || saving} size="icon-sm" variant="ghost" onClick={() => setStatuses((current) => current.filter((_, currentIndex) => currentIndex !== index))}><Trash2 size={14} /></Button>
              </div>
            ))}
          </div>
        </section>
      </SurfaceCard>
    </>
  );
}
