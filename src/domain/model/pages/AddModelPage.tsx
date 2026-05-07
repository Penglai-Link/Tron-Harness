import {
  useMemo,
  useState,
} from "react";
import {
  ArrowLeft,
  Code2,
  Eye,
  EyeOff,
  MoreHorizontal,
  RefreshCw,
  SlidersHorizontal,
  Zap,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  SelectInput,
  SurfaceCard,
  TextInput,
  ToggleSwitch,
  TronShell,
} from "@/shared";

import { useModelCostPreview } from "@/domain/model/hooks";

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

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-mono text-[13px]">{value}</span>
    </div>
  );
}

export function AddModelPage() {
  const costs = useModelCostPreview();
  const [provider, setProvider] = useState("openai");
  const [temperature, setTemperature] = useState(0.7);
  const [tokens, setTokens] = useState(8192);
  const [contextWindow, setContextWindow] = useState("128000");
  const [inheritable, setInheritable] = useState(true);
  const [showKey, setShowKey] = useState(false);
  const [notice, setNotice] = useState("Configuration preview is synced.");

  const preview = useMemo(
    () =>
      JSON.stringify(
        {
          provider,
          model: "gpt-4.1-enterprise",
          temperature,
          maxCompletionTokens: tokens,
          contextWindow,
          inheritable,
        },
        null,
        2,
      ),
    [contextWindow, inheritable, provider, temperature, tokens],
  );

  return (
    <TronShell activeItem="Models" searchPlaceholder="Search models, providers, costs...">
      <div className="-mx-6 -mt-6 mb-6 flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] bg-white px-6">
        <div className="flex items-center gap-3">
          <IconButton label="Back to models" onClick={() => setNotice("Back to Models requested")}>
            <ArrowLeft size={18} />
          </IconButton>
          <h1 className="text-base font-semibold">Configure Model</h1>
        </div>
        <div className="flex items-center gap-2">
          <IconButton label="Refresh" onClick={() => setNotice("Provider metadata refreshed")}>
            <RefreshCw size={17} />
          </IconButton>
          <IconButton label="More actions" onClick={() => setNotice("More model actions opened")}>
            <MoreHorizontal size={17} />
          </IconButton>
          <Button size="sm" onClick={() => setNotice("Deploy Model queued")}>
            <Zap size={15} />
            Deploy Model
          </Button>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-6">
          <SurfaceCard>
            <SectionTitle title="Provider Details" />
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Provider">
                <SelectInput value={provider} onChange={(event) => setProvider(event.target.value)}>
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="vertex">Google Vertex AI</option>
                  <option value="cohere">Cohere</option>
                  <option value="custom">Custom Endpoint</option>
                </SelectInput>
              </Field>
              <Field label="Model Identifier">
                <TextInput defaultValue="gpt-4.1-enterprise" placeholder="e.g., gpt-4-turbo" />
              </Field>
            </div>
            <div className="mt-5 grid gap-4">
              <Field label="API Key" hint="Vault Managed">
                <div className="relative">
                  <TextInput className="w-full pr-10 font-mono" type={showKey ? "text" : "password"} defaultValue="sk-prod-vault-managed-key" />
                  <button className="absolute right-2 top-1/2 -translate-y-1/2 rounded-md p-1 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setShowKey((value) => !value)}>
                    {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
              </Field>
              <Field label="Base URL (Optional)">
                <TextInput defaultValue="https://api.openai.com/v1" />
              </Field>
            </div>
          </SurfaceCard>

          <SurfaceCard>
            <SectionTitle title="Execution Parameters" icon={<SlidersHorizontal size={18} />} />
            <div className="grid gap-5">
              <Field label={`Temperature ${temperature.toFixed(1)}`} hint="Precise (0) to Creative (2)">
                <input className="accent-[var(--color-primary)]" type="range" min={0} max={2} step={0.1} value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} />
              </Field>
              <Field label={`Max Completion Tokens ${tokens.toLocaleString()}`}>
                <input className="accent-[var(--color-primary)]" type="range" min={1024} max={32768} step={1024} value={tokens} onChange={(event) => setTokens(Number(event.target.value))} />
              </Field>
              <Field label="Context Window Limit">
                <SelectInput value={contextWindow} onChange={(event) => setContextWindow(event.target.value)}>
                  <option value="8192">8,192 Tokens</option>
                  <option value="16384">16,384 Tokens</option>
                  <option value="32768">32,768 Tokens</option>
                  <option value="128000">128,000 Tokens</option>
                </SelectInput>
              </Field>
            </div>
          </SurfaceCard>

          <SurfaceCard>
            <SectionTitle title="Access Control" />
            <ToggleSwitch checked={inheritable} label="Inheritable Policy" onCheckedChange={setInheritable} />
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">Allow downstream workspaces and agents to inherit and utilize this model configuration.</p>
          </SurfaceCard>
        </div>

        <div className="space-y-4">
          <SurfaceCard className="sticky top-24">
            <SectionTitle title="Configuration Preview" icon={<Code2 size={18} />} />
            <pre className="max-h-[360px] overflow-auto rounded-lg bg-[var(--color-surface-container-low)] p-4 text-[12px] leading-5">{preview}</pre>
            <div className="mt-5 space-y-3">
              <h3 className="text-sm font-semibold">Estimated Cost</h3>
              {costs.map((line) => (
                <KeyValue key={line.label} label={line.label} value={line.value} />
              ))}
            </div>
            <p className="mt-4 text-[12px] text-[var(--color-on-surface-variant)]">Estimates based on standard public pricing. Negotiated enterprise rates may apply.</p>
            <div className="mt-4">
              <Notice value={notice} />
            </div>
          </SurfaceCard>
        </div>
      </div>
    </TronShell>
  );
}
