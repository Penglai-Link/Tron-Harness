import {
  useEffect,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  List,
  Plus,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  TronShell,
  useToast,
} from "@/shared";

import { createTask } from "@/domain/task/api";
import { createMockTask } from "@/domain/task/mock/data";
import {
  useTaskOptions,
  useTaskQuickReference,
  useTaskWorkspaceOptions,
} from "@/domain/task/hooks";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import type {
  TaskPriority,
  TaskSourceType,
  TaskType,
} from "@/domain/task/model";

const DEFAULT_WORKSPACE_CODE = "TRON_CORE";

function toggleCode(codes: string[], code: string) {
  return codes.includes(code) ? codes.filter((item) => item !== code) : [...codes, code];
}

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

export function CreateTaskPage() {
  const options = useTaskOptions();
  const quickReference = useTaskQuickReference();
  const [workspaceCode, setWorkspaceCode] = useState(DEFAULT_WORKSPACE_CODE);
  const [title, setTitle] = useState("Ship task management v1");
  const [description, setDescription] = useState("");
  const [taskType, setTaskType] = useState<TaskType>("Feature");
  const [sourceType, setSourceType] = useState<TaskSourceType>("Internal");
  const [sourceUrl, setSourceUrl] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("Medium");
  const [agentCodesInput, setAgentCodesInput] = useState("");
  const [appCodesInput, setAppCodesInput] = useState("");
  const [assigneeUserCodes, setAssigneeUserCodes] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const { effectiveUsers, workspaces } = useTaskWorkspaceOptions(workspaceCode);

  useEffect(() => {
    if (!workspaces.length || workspaces.some((workspace) => workspace.workspaceCode === workspaceCode)) return;
    setWorkspaceCode(workspaces[0].workspaceCode);
  }, [workspaceCode, workspaces]);

  useEffect(() => {
    const allowed = new Set(effectiveUsers.map((user) => user.userCode));
    setAssigneeUserCodes((current) => current.filter((userCode) => allowed.has(userCode)));
  }, [effectiveUsers]);

  async function submitTask() {
    setSubmitting(true);
    const payload = {
      agentCodes: splitCodes(agentCodesInput),
      appCodes: splitCodes(appCodesInput),
      assigneeUserCodes,
      description,
      priority,
      sourceType,
      sourceUrl,
      taskType,
      title,
      workspaceCode,
    };

    try {
      const created = dataSource === "api" ? await createTask(payload) : createMockTask(payload);
      toast.success(`Task ${created.taskCode} created`);
      navigate(`/tasks/detail?taskCode=${encodeURIComponent(created.taskCode)}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Task could not be created";
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "New" }]} title="Create Task" />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-6">
          <Field label="Task Title">
            <TextInput aria-label="Task title" className="w-full" value={title} onChange={(event) => setTitle(event.target.value)} />
          </Field>
          <div className="grid gap-6 xl:grid-cols-2">
            <Field label="Workspace">
              <SelectInput aria-label="Workspace" value={workspaceCode} onChange={(event) => setWorkspaceCode(event.target.value)}>
                {workspaces.map((workspace) => (
                  <option key={workspace.workspaceCode} value={workspace.workspaceCode}>
                    {workspace.label}
                  </option>
                ))}
              </SelectInput>
            </Field>
            <Field label="Priority">
              <SelectInput aria-label="Priority" value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>
                {options.priorities.map((item) => <option key={item}>{item}</option>)}
              </SelectInput>
            </Field>
          </div>
          <SurfaceCard>
            <h2 className="mb-3 text-base font-semibold">Classification & Source</h2>
            <div className="grid gap-4 lg:grid-cols-2">
              <Field label="Task Type">
                <SelectInput aria-label="Task Type" value={taskType} onChange={(event) => setTaskType(event.target.value as TaskType)}>
                  {options.taskTypes.map((item) => <option key={item}>{item}</option>)}
                </SelectInput>
              </Field>
              <Field label="Source Type">
                <SelectInput aria-label="Source Type" value={sourceType} onChange={(event) => setSourceType(event.target.value as TaskSourceType)}>
                  {options.sourceTypes.map((item) => <option key={item}>{item}</option>)}
                </SelectInput>
              </Field>
              <Field label="Source URL">
                <TextInput aria-label="Source URL" placeholder="Optional GitHub/GitLab URL" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
              </Field>
              <Field label="Agent Codes">
                <TextInput aria-label="Agent Codes" placeholder="agent-reviewer, agent-builder" value={agentCodesInput} onChange={(event) => setAgentCodesInput(event.target.value)} />
              </Field>
              <Field label="App Codes">
                <TextInput aria-label="App Codes" placeholder="github, notion" value={appCodesInput} onChange={(event) => setAppCodesInput(event.target.value)} />
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <h2 className="mb-3 text-base font-semibold">Assigned Users</h2>
            <p className="mb-4 text-sm text-[var(--color-on-surface-variant)]">Only effective users in the selected workspace can be assigned.</p>
            <div className="grid gap-2 md:grid-cols-2">
              {effectiveUsers.map((user) => (
                <label className="flex items-start gap-3 rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={user.userCode}>
                  <input
                    aria-label={`Assign ${user.displayName}`}
                    checked={assigneeUserCodes.includes(user.userCode)}
                    className="mt-1"
                    type="checkbox"
                    onChange={() => setAssigneeUserCodes((current) => toggleCode(current, user.userCode))}
                  />
                  <span>
                    <span className="block font-medium">{user.displayName}</span>
                    <span className="block text-xs text-[var(--color-on-surface-variant)]">{user.userCode}</span>
                  </span>
                </label>
              ))}
              {!effectiveUsers.length ? <p className="text-sm text-[var(--color-on-surface-variant)]">No effective users available for this workspace.</p> : null}
            </div>
          </SurfaceCard>
          <SurfaceCard className="p-0">
            <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-4 py-3">
              <div>
                <h2 className="text-base font-semibold">Description</h2>
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">Markdown supported</p>
              </div>
              <Button variant="secondary" onClick={() => toast.info("Editor toolbar is a placeholder in v1")}>
                <List size={15} />
                Editor Tools
              </Button>
            </div>
            <TextArea aria-label="Task description" className="min-h-[260px] w-full rounded-none border-0 bg-white p-5" placeholder="Describe the work, expected output, context, and constraints..." value={description} onChange={(event) => setDescription(event.target.value)} />
          </SurfaceCard>
        </div>
        <aside className="space-y-4">
          <SurfaceCard>
            <h2 className="mb-3 text-base font-semibold">Quick Reference</h2>
            <ul className="space-y-2 text-sm text-[var(--color-on-surface-variant)]">
              {quickReference.tips.map((tip) => <li key={tip}>{tip}</li>)}
            </ul>
            <div className="mt-5 flex flex-wrap gap-2">
              {quickReference.variables.map((variable) => <Badge key={variable} tone="neutral">{variable}</Badge>)}
            </div>
          </SurfaceCard>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => navigate("/tasks")}>Cancel</Button>
            <Button disabled={submitting || !title.trim() || !workspaceCode} onClick={submitTask}>
              <Plus size={15} />
              Create Task
            </Button>
          </div>
        </aside>
      </div>
    </TronShell>
  );
}
