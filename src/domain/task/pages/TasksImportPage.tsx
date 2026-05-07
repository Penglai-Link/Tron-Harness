import { useState } from "react";
import {
  Link,
  useNavigate,
} from "react-router";
import {
  Check,
  FolderOpen,
  Plus,
  Upload,
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

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

export function TasksImportPage() {
  const [source, setSource] = useState("gitlab");
  const [status, setStatus] = useState("Select a source and start the import.");
  const navigate = useNavigate();

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search task imports...">
      <PageHeader
        breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "Import" }]}
        title="Import Task"
        description="Import issues or work items into the task workflow, then review them as Tron tasks."
        status="Integration"
        actions={
          <>
            <Button asChild variant="secondary">
              <Link to="/tasks">Back to Board</Link>
            </Button>
            <Button onClick={() => setStatus(`${source.toUpperCase()} import scanned 4 candidate tasks`)}>
              <Upload size={16} aria-hidden="true" />
              Start Import
            </Button>
          </>
        }
      />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
        <SurfaceCard className="space-y-5">
          <Field label="Import Source">
            <SelectInput value={source} onChange={(event) => setSource(event.target.value)}>
              <option value="gitlab">GitLab Issue</option>
              <option value="github">GitHub Issue</option>
              <option value="csv">CSV Backlog</option>
            </SelectInput>
          </Field>
          <Field label="Source URL or Query">
            <TextInput placeholder="https://gitlab.example.com/group/project/-/issues?label=agent-ready" />
          </Field>
          <Field label="Target App">
            <SelectInput defaultValue="tron-console">
              <option value="tron-console">Tron Console</option>
              <option value="memory-service">Memory Service</option>
              <option value="agent-runtime">Agent Runtime</option>
            </SelectInput>
          </Field>
          <Field label="Import Notes">
            <TextArea className="min-h-40" placeholder="Map labels, priority, assignee, and execution constraints before creating tasks." />
          </Field>
        </SurfaceCard>
        <aside className="space-y-4">
          <SurfaceCard>
            <div className="mb-4 flex items-center gap-2 border-b border-[var(--color-outline-variant)] pb-3">
              <FolderOpen size={16} aria-hidden="true" />
              <h2 className="text-base font-semibold">Import Preview</h2>
            </div>
            <div className="space-y-3 text-sm">
              {["Normalize issue metadata", "Map assignee to agent", "Attach source link", "Prepare task draft"].map((item) => (
                <div className="flex items-center gap-2" key={item}>
                  <Check size={15} className="text-[var(--color-success)]" aria-hidden="true" />
                  <span>{item}</span>
                </div>
              ))}
            </div>
          </SurfaceCard>
          <Notice value={status} />
          <Button className="w-full" onClick={() => navigate("/tasks/new")}>
            <Plus size={16} aria-hidden="true" />
            Create Task from Import
          </Button>
        </aside>
      </div>
    </TronShell>
  );
}
