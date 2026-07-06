import { useState } from "react";
import { useNavigate } from "react-router";
import {
  Code2,
  Layers3,
  Play,
  Save,
  Settings2,
  SlidersHorizontal,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
import {
  Field,
  PageHeader,
  SegmentedTabs,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { usePromptVariableList } from "@/domain/prompt/hooks";
import {
  promptShellNavItems,
  promptVisibilityOptions,
  promptWorkspaceTabs,
} from "@/domain/prompt/constants";
import { SelectItem } from "@/components/ui/select";

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

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-mono text-[13px]">{value}</span>
    </div>
  );
}


export function CreatePromptPage() {
  const navigate = useNavigate();
  const variables = usePromptVariableList();
  const [tab, setTab] = useState("prompts");
  const [navItem, setNavItem] = useState("Library");
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [visibility, setVisibility] = useState("Workspace");
  const [temperature, setTemperature] = useState(0.7);
  const [notice, setNotice] = useState("");
  const toggleNotifications = () => {
    const nextOpen = !notificationsOpen;
    setNotificationsOpen(nextOpen);
    setNotice(nextOpen ? "Notifications panel opened" : "Notifications closed");
  };

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Prompts", href: "/prompts" }, { label: "New" }]}
        title="Create New Prompt"
        actions={
          <>
            <Button variant="secondary" onClick={() => navigate("/prompts")}>
              Cancel
            </Button>
            <Button variant="secondary" onClick={() => setNotice("Prompt test queued")}>
              <Play size={15} />
              Test Prompt
            </Button>
            <Button onClick={() => setNotice("Prompt saved")}>
              <Save size={15} />
              Save
            </Button>
          </>
        }
      />
      <SurfaceCard className="mb-6 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap gap-2">
            {promptShellNavItems.map((item) => (
              <button
                aria-pressed={navItem === item}
                className={cn("inline-flex h-9 items-center gap-2 rounded-lg px-3 text-sm", navItem === item ? "bg-[var(--color-surface-container-high)] font-medium" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]")}
                data-selected={navItem === item ? "true" : "false"}
                type="button"
                key={item}
                onClick={() => {
                  setNavItem(item);
                  setNotice(`${item} navigation selected`);
                }}
              >
                <Layers3 size={15} />
                {item}
              </button>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <SegmentedTabs value={tab} onValueChange={setTab} items={promptWorkspaceTabs} />
            <div className="relative">
              <IconButton label="Notifications" onClick={toggleNotifications}>
                <Settings2 size={16} />
              </IconButton>
              {notificationsOpen ? (
                <div className="absolute right-0 top-10 z-40 w-72 rounded-xl border border-[var(--color-outline-variant)] bg-white p-3 text-sm shadow-lg">
                  <p className="font-semibold">Notifications</p>
                  <p className="mt-1 text-[var(--color-on-surface-variant)]">3 prompt workflow alerts are ready for review.</p>
                  <button className="mt-3 rounded-lg bg-[var(--color-primary)] px-3 py-1.5 text-xs font-semibold text-white" type="button" onClick={() => setNotice("Notification center opened")}>
                    Open center
                  </button>
                </div>
              ) : null}
            </div>
          </div>
        </div>
      </SurfaceCard>
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-6">
          <SurfaceCard>
            <div className="grid gap-4 md:grid-cols-3">
              <Field label="Prompt Name">
                <TextInput placeholder="e.g., Customer Support Response Generator" />
              </Field>
              <Field label="Category">
                <SelectInput defaultValue="System">
                  <SelectItem>System</SelectItem>
                  <SelectItem>Persona</SelectItem>
                  <SelectItem>Tool</SelectItem>
                </SelectInput>
              </Field>
              <Field label="Visibility">
                <div className="flex h-10 items-center gap-4">
                  {promptVisibilityOptions.map((item) => (
                    <label className="flex items-center gap-2 text-sm" key={item}>
                      <input checked={visibility === item} name="visibility" type="radio" onChange={() => setVisibility(item)} />
                      {item}
                    </label>
                  ))}
                </div>
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Prompt Template" />
            <div className="grid grid-cols-[44px_1fr] overflow-hidden rounded-lg border border-[var(--color-outline-variant)]">
              <div className="bg-[var(--color-surface-container-low)] py-3 text-center font-mono text-[12px] leading-6 text-[var(--color-on-surface-variant)]">1<br />2<br />3<br />4<br />5<br />6<br />7<br />8<br />9<br />10</div>
              <TextArea className="min-h-[260px] rounded-none border-0 font-mono leading-6" defaultValue={"You are a Tron workspace agent.\n\nSummarize {{incident_summary}} for a {{customer_tier}} customer.\n\nReturn actions, risks, and required owner."} />
            </div>
          </SurfaceCard>
        </div>
        <div className="space-y-4">
          <SurfaceCard>
            <SectionTitle title="Model" icon={<SlidersHorizontal size={17} />} />
            <Field label="Model">
              <SelectInput defaultValue="GPT-4o">
                <SelectItem>GPT-4o</SelectItem>
                <SelectItem>Claude 3.5 Sonnet</SelectItem>
                <SelectItem>Llama 3 70B</SelectItem>
              </SelectInput>
            </Field>
            <Field label={`Temperature ${temperature.toFixed(1)}`} hint="Precise to Creative">
              <input className="accent-[var(--color-primary)]" type="range" min={0} max={2} step={0.1} value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} />
            </Field>
            <KeyValue label="Input (Template)" value="~450" />
            <KeyValue label="Max Output" value="2,048" />
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Detected Variables" icon={<Code2 size={17} />} />
            <div className="space-y-2">
              {variables.map((variable) => (
                <div className="rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={variable.name}>
                  <div className="flex justify-between">
                    <span className="font-mono">{variable.name}</span>
                    <Badge tone="info">{variable.type}</Badge>
                  </div>
                  <TextInput className="mt-2 h-8 w-full" defaultValue={variable.sample} />
                </div>
              ))}
            </div>
            <div className="mt-4">
              <Notice value={notice ? `${notice} Visibility: ${visibility}. Top tab: ${tab}.` : ""} />
            </div>
          </SurfaceCard>
        </div>
      </div>
    </>
  );
}
