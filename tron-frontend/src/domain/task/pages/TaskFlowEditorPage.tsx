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
      <div className="max-w-4xl space-y-4">
        <SurfaceCard className="grid gap-4 p-5 md:grid-cols-2">
          <Field label={flowT("editor.code", "Code")}><TextInput disabled={editing} value={flowCode} onChange={(event) => setFlowCode(event.target.value)} /></Field>
          <Field label={flowT("editor.name", "Name")}><TextInput value={name} onChange={(event) => setName(event.target.value)} /></Field>
        </SurfaceCard>
        <SurfaceCard className="p-5">
          <div className="mb-3 flex items-center justify-between"><h2 className="font-semibold">{flowT("editor.statuses", "Statuses")}</h2><Button size="sm" variant="secondary" onClick={() => setStatuses((current) => [...current.slice(0, -1), { statusCode: "", name: "", required: false }, current[current.length - 1]])}>{flowT("actions.addStatus", "Add")}</Button></div>
          <div className="space-y-2">
            {statuses.map((status, index) => (
              <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_40px] gap-2" key={`${status.statusCode}-${index}`}>
                <TextInput disabled={status.required} placeholder={flowT("editor.statusCode", "Status code")} value={status.statusCode} onChange={(event) => updateStatus(index, "statusCode", event.target.value)} />
                <TextInput disabled={status.required} placeholder={flowT("editor.statusName", "Status name")} value={status.required ? taskFlowStatusDisplayName(status, flowT) : status.name} onChange={(event) => updateStatus(index, "name", event.target.value)} />
                <Button aria-label={flowT("aria.deleteStatus", "Delete status")} disabled={status.required} size="sm" variant="ghost" onClick={() => setStatuses((current) => current.filter((_, currentIndex) => currentIndex !== index))}>{flowT("aria.deleteStatus", "Delete status")}</Button>
              </div>
            ))}
          </div>
        </SurfaceCard>
      </div>
    </>
  );
}
