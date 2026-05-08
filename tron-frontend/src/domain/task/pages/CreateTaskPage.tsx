import { useState } from "react";
import { useNavigate } from "react-router";
import {
  Code2,
  Link,
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
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { useTaskQuickReference } from "@/domain/task/hooks";
import { taskPriorityOptions } from "@/domain/task/constants";

function IconButton({ label, children, onClick }: { label: string; children: React.ReactNode; onClick?: () => void }) {
  return (
    <button className="inline-flex size-8 items-center justify-center rounded-lg text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]" type="button" aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function SectionTitle({ icon, title }: { icon?: React.ReactNode; title: string }) {
  return (
    <div className="mb-4 flex items-center gap-2 border-b border-[var(--color-outline-variant)] pb-3">
      {icon}
      <h2 className="text-base font-semibold">{title}</h2>
    </div>
  );
}

export function CreateTaskPage() {
  const quickReference = useTaskQuickReference();
  const [priority, setPriority] = useState("Med");
  const [notice, setNotice] = useState("");
  const navigate = useNavigate();

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "New" }]} title="Create Task" />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
        <div className="space-y-6">
          <Field label="Task Title">
            <TextInput className="w-full" placeholder="e.g., Analyze Q3 User Engagement Metrics" />
          </Field>
          <div className="grid gap-6 xl:grid-cols-3">
            <Field label="Assign Agent">
              <SelectInput defaultValue="">
                <option value="" disabled>
                  Select an agent...
                </option>
                <option>Nexus (Data Analyst)</option>
                <option>Cipher (Security)</option>
                <option>Atlas (Research)</option>
                <option>Unassigned</option>
              </SelectInput>
            </Field>
            <Field label="Associated App">
              <SelectInput defaultValue="">
                <option value="" disabled>
                  Select application...
                </option>
                <option>Snowflake DB</option>
                <option>Salesforce CRM</option>
                <option>Zendesk Support</option>
                <option>GitHub</option>
              </SelectInput>
            </Field>
            <Field label="Priority">
              <div className="grid h-10 grid-cols-3 gap-2">
                {taskPriorityOptions.map((item) => (
                  <button className={cn("rounded-lg border border-[var(--color-outline-variant)] text-sm", priority === item ? "bg-[var(--color-surface-variant)] font-medium text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]")} type="button" key={item} onClick={() => setPriority(item)}>
                    {item}
                  </button>
                ))}
              </div>
            </Field>
          </div>
          <SurfaceCard className="p-0">
            <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-4 py-3">
              <div>
                <h2 className="text-base font-semibold">Description</h2>
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">Markdown supported</p>
              </div>
              <div className="flex gap-1">
                {[List, Link, Code2].map((Icon, index) => (
                  <IconButton label={`Editor tool ${index + 1}`} key={index} onClick={() => setNotice(`Editor tool ${index + 1} applied`)}>
                    <Icon size={15} />
                  </IconButton>
                ))}
              </div>
            </div>
            <TextArea className="min-h-[360px] rounded-none border-0 bg-white p-5" placeholder="Describe the work, expected output, context, and constraints..." />
          </SurfaceCard>
        </div>
        <aside className="space-y-4">
          <SurfaceCard>
            <SectionTitle title="Quick Reference" />
            <h3 className="mb-2 text-sm font-semibold">Tips for Agents</h3>
            <ul className="space-y-2 text-sm text-[var(--color-on-surface-variant)]">
              {quickReference.tips.map((tip) => (
                <li key={tip}>{tip}</li>
              ))}
            </ul>
            <h3 className="mb-2 mt-5 text-sm font-semibold">Recent Variables</h3>
            <div className="flex flex-wrap gap-2">
              {quickReference.variables.map((variable) => (
                <Badge key={variable} tone="neutral">
                  {variable}
                </Badge>
              ))}
            </div>
          </SurfaceCard>
          <Notice value={notice ? `${notice} Priority: ${priority}.` : ""} />
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => navigate("/tasks")}>
              Cancel
            </Button>
            <Button onClick={() => {
              setNotice("Task creation submitted");
              navigate("/tasks/detail");
            }}>
              <Plus size={15} />
              Create Task
            </Button>
          </div>
        </aside>
      </div>
    </TronShell>
  );
}
