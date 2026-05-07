import { useState } from "react";
import {
  CircleCheck,
  Copy,
  Wrench,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  InspectorPanel,
  PageHeader,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
  TronShell,
} from "@/shared";

import { useSkillSchemaPreview } from "@/domain/skill/api";

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

export function CreateSkillPage() {
  const schema = useSkillSchemaPreview();
  const [enabled, setEnabled] = useState(true);
  const [inheritable, setInheritable] = useState(true);
  const [notice, setNotice] = useState("Valid JSON Schema");
  const jsonSchema = JSON.stringify({ name: schema.name, parameters: Object.fromEntries(schema.fields.map((field) => [field.name, field.type])) }, null, 2);

  return (
    <TronShell
      activeItem="Skills"
      searchPlaceholder="Search skills..."
      rightPanel={
        <InspectorPanel title="Schema Preview">
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--color-success)]">
              <CircleCheck size={16} />
              {notice}
            </div>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">The schema is properly formatted and ready to be parsed by the agent engine.</p>
          </SurfaceCard>
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold">
              <Wrench size={16} />
              {schema.name}
            </h3>
            {schema.fields.map((field) => (
              <KeyValue key={field.name} label={field.name} value={`${field.type}${field.required ? " required" : ""}`} />
            ))}
          </SurfaceCard>
        </InspectorPanel>
      }
    >
      <PageHeader
        breadcrumbs={[{ label: "Skills", href: "/skills" }, { label: "New" }]}
        title="Create New Skill"
        description="Define a new capability for agents using JSON Schema."
        actions={
          <Button variant="secondary" onClick={() => setNotice("Template loaded")}>
            Load Template
          </Button>
        }
      />
      <div className="mx-auto max-w-3xl space-y-6 pb-24">
        <SurfaceCard>
          <SectionTitle title="General Details" />
          <div className="grid gap-4">
            <Field label="Skill Name">
              <TextInput placeholder="e.g., Get Weather, Search Knowledge Base" />
            </Field>
            <Field label="Description (Optional)">
              <TextArea placeholder="Describe what this skill does and when agents should use it." />
            </Field>
          </div>
        </SurfaceCard>
        <SurfaceCard>
          <SectionTitle title="Tool Definition" />
          <div className="mb-2 flex items-center justify-between">
            <span className="text-sm font-medium">schema.json</span>
            <Button size="sm" variant="secondary" onClick={() => setNotice("Schema copied")}>
              <Copy size={14} />
              Copy
            </Button>
          </div>
          <TextArea className="min-h-[260px] font-mono text-[13px]" defaultValue={jsonSchema} />
        </SurfaceCard>
        <SurfaceCard>
          <SectionTitle title="Configuration" />
          <div className="space-y-4">
            <ToggleSwitch checked={enabled} label="Enable by Default" onCheckedChange={setEnabled} />
            <ToggleSwitch checked={inheritable} label="Inheritable" onCheckedChange={setInheritable} />
          </div>
        </SurfaceCard>
      </div>
      <div className="fixed bottom-0 left-[280px] right-0 z-30 border-t border-[var(--color-outline-variant)] bg-white/95 px-6 py-3 backdrop-blur">
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setNotice("Skill creation cancelled")}>
            Cancel
          </Button>
          <Button onClick={() => setNotice("Skill creation submitted")}>Create Skill</Button>
        </div>
      </div>
    </TronShell>
  );
}
