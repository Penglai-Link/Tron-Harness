import { useState } from "react";
import {
  ArrowLeft,
  Upload,
  X,
} from "lucide-react";

import { Link } from "react-router";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { importApplicationSourceOptions } from "@/domain/app/constants";
import { useImportApplicationDefaultsBySource } from "@/domain/app/hooks";
import type { ImportApplicationDefaults } from "@/domain/app/model";

function RoutePageShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
  return (
    <TronShell activeItem={activeItem} searchPlaceholder="Search Tron...">
      {children}
    </TronShell>
  );
}

function ImportApplicationPage({ initialSource }: { initialSource: ImportApplicationDefaults["source"] }) {
  const defaults = useImportApplicationDefaultsBySource(initialSource);
  const [source, setSource] = useState(defaults.source);
  const [form, setForm] = useState(defaults);
  const [tags, setTags] = useState(defaults.tags);
  const [feedback, setFeedback] = useState("Ready to import");

  function update(field: keyof ImportApplicationDefaults, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
  }

  return (
    <RoutePageShell activeItem="Apps">
      <div className="mx-auto max-w-3xl">
        {initialSource === "local" ? (
          <Link className="mb-6 inline-flex items-center gap-1 text-sm text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]" to="/apps">
            <ArrowLeft size={16} aria-hidden="true" />
            Back to Apps
          </Link>
        ) : null}
        <SurfaceCard className="overflow-hidden p-0">
          <div className="border-b border-[var(--color-outline-variant)] px-8 py-6">
            <h1 className="text-2xl font-semibold">Import Application</h1>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">Connect a repository or upload local files to deploy a new app.</p>
          </div>
          <div className="space-y-6 p-8">
            <div>
              <p className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Source</p>
              <div className="grid gap-3 sm:grid-cols-3">
                {importApplicationSourceOptions.map(({ icon: Icon, label, value }) => (
                  <button
                    className={cn(
                      "flex flex-col items-center justify-center rounded-lg border p-4 text-sm transition-colors",
                      source === value ? "border-2 border-[var(--color-primary)] text-[var(--color-primary)]" : "border-[var(--color-outline-variant)] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
                    )}
                    key={value}
                    type="button"
                    onClick={() => {
                      setSource(value);
                      update("source", value);
                    }}
                  >
                    <Icon size={24} aria-hidden="true" />
                    <span className="mt-2">{label}</span>
                  </button>
                ))}
              </div>
            </div>

            {source !== "local" ? (
              <>
                <Field label="Repository URL">
                  <TextInput value={form.repositoryUrl} placeholder="https://github.com/username/repo" onChange={(event) => update("repositoryUrl", event.target.value)} />
                </Field>
                <Field label="Access Token (Private Repos)" hint="Optional">
                  <TextInput type="password" value={form.accessToken} placeholder="ghp_xxxxxxxxxxxxxxxxxxxx" onChange={(event) => update("accessToken", event.target.value)} />
                </Field>
              </>
            ) : null}
            <Field label="Local File System Path" hint={source !== "local" ? "Directory where the repository will be cloned." : undefined}>
              <TextInput value={form.localPath} placeholder="/Users/username/projects/my-app" onChange={(event) => update("localPath", event.target.value)} />
            </Field>
            <Field label="App Name">
              <TextInput value={form.appName} placeholder="my-awesome-app" onChange={(event) => update("appName", event.target.value)} />
            </Field>
            <Field label="Tags">
              <div className="flex min-h-10 flex-wrap items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white p-2">
                {tags.map((tag) => (
                  <span className="inline-flex items-center gap-1 rounded-full bg-[var(--color-accent-container)] px-2.5 py-1 text-[13px] text-[var(--color-on-accent-container)]" key={tag}>
                    {tag}
                    <button type="button" onClick={() => setTags((current) => current.filter((item) => item !== tag))}>
                      <X size={13} aria-hidden="true" />
                    </button>
                  </span>
                ))}
                <input className="min-w-[120px] flex-1 bg-transparent text-sm outline-none" placeholder="Add tags..." onKeyDown={(event) => {
                  if (event.key === "Enter" && event.currentTarget.value.trim()) {
                    setTags((current) => [...current, event.currentTarget.value.trim()]);
                    event.currentTarget.value = "";
                  }
                }} />
              </div>
            </Field>
            {source !== "local" ? (
              <Field label="Branch / Ref">
                <TextInput value={form.branch} onChange={(event) => update("branch", event.target.value)} />
              </Field>
            ) : null}
          </div>
          <div className="flex items-center justify-between border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-8 py-5">
            <span className="text-[13px] text-[var(--color-on-surface-variant)]">{feedback}</span>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={() => setFeedback("Import canceled")}>Cancel</Button>
              <Button onClick={() => setFeedback(`Import queued for ${form.appName || "new app"}`)}>
                <Upload size={16} aria-hidden="true" />
                Import App
              </Button>
            </div>
          </div>
        </SurfaceCard>
      </div>
    </RoutePageShell>
  );
}

export function ImportApplicationGithubPage() {
  return <ImportApplicationPage initialSource="github" />;
}
