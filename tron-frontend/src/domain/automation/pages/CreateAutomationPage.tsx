import { useState } from "react";
import {
  Database,
  GitBranch,
  Plus,
  Trash2,
  Zap,
} from "lucide-react";

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
import { defaultAutomationPrerequisites } from "@/domain/automation/constants";

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

export function CreateAutomationPage() {
  const [triggerType, setTriggerType] = useState("Scheduled");
  const [cron, setCron] = useState("0 0 * * *");
  const [prereqs, setPrereqs] = useState<string[]>([...defaultAutomationPrerequisites]);
  const [notice, setNotice] = useState("");

  return (
    <TronShell activeItem="Automations" searchPlaceholder="Search automations...">
      <PageHeader
        breadcrumbs={[{ label: "Automations", href: "/automations" }, { label: "New" }]}
        title="Create Automation"
        actions={
          <Button onClick={() => setNotice("Create automation submitted")}>
            <Plus size={15} />
            Create
          </Button>
        }
      />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <SurfaceCard>
            <SectionTitle title="Basic Info" />
            <div className="grid gap-4">
              <Field label="Automation Name">
                <TextInput placeholder="e.g., Daily Sync Job" />
              </Field>
              <Field label="Description (Optional)">
                <TextArea placeholder="Describe what this automation does..." />
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Trigger Configuration" icon={<Zap size={17} />} />
            <div className="grid gap-4">
              <Field label="Trigger Type">
                <SelectInput value={triggerType} onChange={(event) => setTriggerType(event.target.value)}>
                  <option>Scheduled</option>
                  <option>Webhook</option>
                  <option>Event-driven</option>
                </SelectInput>
              </Field>
              <Field label="Schedule Definition">
                <div className="flex gap-2">
                  <TextInput className="flex-1 font-mono" value={cron} onChange={(event) => setCron(event.target.value)} />
                  <Button variant="secondary" onClick={() => setNotice(`Parsed as CRON: ${cron}`)}>
                    Parse
                  </Button>
                </div>
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Application & Agent" icon={<Database size={17} />} />
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Target Application">
                <SelectInput defaultValue="">
                  <option value="" disabled>
                    Select App...
                  </option>
                  <option>Salesforce Data Sync</option>
                  <option>AWS S3 Bucket Monitor</option>
                  <option>Internal HR Portal</option>
                </SelectInput>
              </Field>
              <Field label="Assigned Agent">
                <SelectInput defaultValue="">
                  <option value="" disabled>
                    Select Agent...
                  </option>
                  <option>Agent Alpha (US-East)</option>
                  <option>Agent Beta (EU-West)</option>
                  <option>Global Pool</option>
                </SelectInput>
              </Field>
            </div>
          </SurfaceCard>
        </div>
        <SurfaceCard>
          <SectionTitle title="Dependencies" icon={<GitBranch size={17} />} />
          <div className="space-y-5">
            <div>
              <div className="mb-2 flex items-center justify-between">
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Pre-requisite Tasks</p>
                <Button size="sm" variant="secondary" onClick={() => setPrereqs((current) => [...current, `New prerequisite ${current.length + 1}`])}>
                  <Plus size={14} />
                  Add Task
                </Button>
              </div>
              <div className="space-y-2">
                {prereqs.map((task) => (
                  <div className="flex items-center justify-between rounded-lg border border-[var(--color-outline-variant)] px-3 py-2 text-sm" key={task}>
                    <span>{task}</span>
                    <IconButton label={`Remove ${task}`} onClick={() => setPrereqs((current) => current.filter((item) => item !== task))}>
                      <Trash2 size={14} />
                    </IconButton>
                  </div>
                ))}
              </div>
            </div>
            <Button variant="secondary" onClick={() => setNotice("Post-execution action added")}>
              <Plus size={14} />
              Add Action
            </Button>
            <Notice value={notice ? `${triggerType}. ${notice}` : ""} />
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}
