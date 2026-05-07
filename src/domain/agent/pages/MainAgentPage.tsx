import { useState } from "react";
import {
  Bot,
  Minus,
  Plus,
  Send,
  Terminal,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  MetricCard,
  SurfaceCard,
  TextArea,
  TronShell,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { useMainAgentMessages } from "@/domain/agent/hooks";

function RoutePageShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
  return (
    <TronShell activeItem={activeItem} searchPlaceholder="Search Tron...">
      {children}
    </TronShell>
  );
}

export function MainAgentPage() {
  const messages = useMainAgentMessages();
  const [open, setOpen] = useState(true);
  const [minimized, setMinimized] = useState(false);
  const [draft, setDraft] = useState("");
  const [status, setStatus] = useState("Executing... 0.8s");

  return (
    <RoutePageShell activeItem="Dashboard">
      <div className="relative min-h-[760px]">
        <div className="grid gap-4 opacity-55 lg:grid-cols-3">
          <MetricCard label="System Health" value="99.98%" trend="All services operational" />
          <MetricCard label="Active Agents" value="42" trend="+3 since last hour" />
          <MetricCard label="Pending Tasks" value="1,204" trend="Avg processing time: 1.2s" />
          <SurfaceCard className="lg:col-span-2"><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Throughput Analytics</h2><div className="h-48 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]" /></SurfaceCard>
          <SurfaceCard><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Recent Activity</h2><p className="text-sm">Deployment triggered in us-east-1</p></SurfaceCard>
        </div>
        <div className="absolute inset-0 bg-[rgba(253,248,248,0.45)]" />
        {open ? (
          <aside className={cn("fixed bottom-6 right-6 z-50 flex w-[440px] max-w-[calc(100vw-3rem)] flex-col overflow-hidden rounded-[20px] border border-[var(--color-outline-variant)] bg-white shadow-[0_8px_32px_rgba(0,0,0,0.12)]", minimized ? "h-[72px]" : "h-[680px]")}>
            <div className="flex h-[60px] items-center justify-between border-b border-[var(--color-outline-variant)] px-5">
              <div className="flex items-center gap-3">
                <span className="relative flex size-8 items-center justify-center rounded-full bg-[var(--color-primary)] text-white"><Bot size={18} /><i className="absolute bottom-0 right-0 size-2.5 rounded-full border-2 border-white bg-green-500" /></span>
                <span><span className="block text-[15px] font-semibold">Main Agent</span><span className="block text-[11px] text-[var(--color-on-surface-variant)]">Workspace Coordinator</span></span>
              </div>
              <div className="flex gap-1">
                <Button size="icon" variant="ghost" onClick={() => setMinimized((current) => !current)}><Minus size={16} /></Button>
                <Button size="icon" variant="ghost" onClick={() => setOpen(false)}><X size={16} /></Button>
              </div>
            </div>
            {!minimized ? (
              <>
                <div className="flex-1 space-y-5 overflow-y-auto bg-[#fcfafa] p-5">
                  <div className="text-center"><Badge>Today, 14:02</Badge></div>
                  {messages.map((message) => <div className={cn("rounded-2xl px-4 py-3 text-sm", message.role === "user" ? "ml-auto max-w-[85%] bg-[var(--color-surface-container)]" : "max-w-[95%] border border-[var(--color-outline-variant)] bg-white")} key={message.id}>{message.body}</div>)}
                  <SurfaceCard className="p-3">
                    <h3 className="mb-2 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Reasoning Process</h3>
                    <ol className="list-inside list-decimal space-y-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                      <li>Fetch metrics for primary clusters.</li>
                      <li>Evaluate CPU and Memory over last 15m.</li>
                      <li>Calculate standard node scale-up.</li>
                    </ol>
                  </SurfaceCard>
                  <div className="overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white">
                    <div className="flex items-center justify-between bg-[var(--color-surface-container)] px-3 py-2 text-sm"><span className="flex items-center gap-2"><Terminal size={14} />FetchClusterMetrics</span><Badge>{status}</Badge></div>
                    <pre className="overflow-x-auto bg-[#1e1e1e] p-3 text-[11px] text-gray-300">{"> GET /api/v2/metrics/clusters?tags=primary\n{\n  \"us-east-1-alpha\": { \"cpu\": \"84%\", \"mem\": \"72%\" },\n  \"eu-west-1-beta\":  { \"cpu\": \"65%\", \"mem\": \"50%\" }\n}"}</pre>
                  </div>
                  <div className="flex gap-2">
                    <Button size="sm" onClick={() => setStatus("Scale-up approved")}>Execute Scale-up</Button>
                    <Button size="sm" variant="secondary" onClick={() => setStatus("Plan review opened")}>Review Plan</Button>
                  </div>
                </div>
                <div className="border-t border-[var(--color-outline-variant)] p-4">
                  <div className="flex items-end gap-2 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1.5">
                    <Button size="icon" variant="ghost" aria-label="Open attachment picker" onClick={() => setStatus("Attachment picker opened")}><Plus size={18} /></Button>
                    <TextArea className="min-h-9 flex-1 border-0 bg-transparent" rows={1} placeholder="Message Main Agent..." value={draft} onChange={(event) => setDraft(event.target.value)} />
                    <Button size="icon" onClick={() => setDraft("")}><Send size={16} /></Button>
                  </div>
                </div>
              </>
            ) : null}
          </aside>
        ) : (
          <Button className="fixed bottom-6 right-6 z-50" onClick={() => setOpen(true)}><Bot size={16} />Open Main Agent</Button>
        )}
      </div>
    </RoutePageShell>
  );
}
