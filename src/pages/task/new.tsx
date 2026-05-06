import { useMemo, useState } from "react";
import {
  ArrowLeft,
  Bot,
  ChevronLeft,
  ChevronRight,
  CircleCheck,
  ClipboardList,
  Code2,
  Copy,
  Database,
  Eye,
  EyeOff,
  FileText,
  Filter,
  GitBranch,
  Layers3,
  Link,
  List,
  MoreHorizontal,
  Paperclip,
  Play,
  Plus,
  RefreshCw,
  Save,
  Search,
  Send,
  Settings2,
  SlidersHorizontal,
  Terminal,
  Trash2,
  Wrench,
  Zap,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable, Field, InspectorPanel, PageHeader, SegmentedTabs, SelectInput, SurfaceCard, TextArea, TextInput, ToggleSwitch, TronShell } from "@/components/tron";
import { useNavigate } from "@/lib/client-router";
import { cn } from "@/lib/utils";
import {
  getApps,
  getAuditLogs,
  getAutomations,
  getChatMessages,
  getChatSessionGroups,
  getModelCostPreview,
  getPromptVariables,
  getSkillSchemaPreview,
  getTaskQuickReference,
  type AppRecord,
  type AuditLogRecord,
  type AutomationRecord,
  type BadgeStatus,
  type ChatMessage,
} from "@/services/console/agent-workflow-data";

function IconButton({ label, children, onClick }: { label: string; children: React.ReactNode; onClick?: () => void }) {
  return (
    <button className="inline-flex size-8 items-center justify-center rounded-lg text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]" type="button" aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
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
  const quickReference = useMemo(() => getTaskQuickReference(), []);
  const [priority, setPriority] = useState("Med");
  const [notice, setNotice] = useState("Draft task ready.");
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
                {["Low", "Med", "High"].map((item) => (
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
          <Notice value={`${notice} Priority: ${priority}.`} />
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
