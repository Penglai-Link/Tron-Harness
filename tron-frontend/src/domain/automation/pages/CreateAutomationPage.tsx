import {
  useEffect,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Maximize2,
  Save,
  X,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { createAutomation } from "@/domain/automation/api";
import { editableAutomationStatuses } from "@/domain/automation/constants";
import { createMockAutomation } from "@/domain/automation/mock/data";
import type { AutomationOperationalStatus } from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function CreateAutomationPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("automation");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [cronExpression, setCronExpression] = useState("0 9 * * *");
  const [status, setStatus] = useState<AutomationOperationalStatus>("Active");
  const [prompt, setPrompt] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [promptFullscreenOpen, setPromptFullscreenOpen] = useState(false);

  useEffect(() => {
    if (!promptFullscreenOpen) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPromptFullscreenOpen(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [promptFullscreenOpen]);

  async function submitAutomation() {
    if (submitting) return;
    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before creating an automation");
      return;
    }

    setSubmitting(true);

    try {
      const created = dataSource === "api"
        ? await createAutomation({
          cronExpression,
          description,
          name,
          prompt,
          status,
          workspaceCode: activeWorkspaceCode,
        })
        : createMockAutomation({
          cronExpression,
          description,
          name,
          prompt,
          status,
          workspaceCode: activeWorkspaceCode,
        });
      toast.success(`Automation ${created.automationCode} created`);
      navigate(`/automations/detail?automationCode=${encodeURIComponent(created.automationCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Automations", href: "/automations" }, { label: "New" }]}
        title="Create Automation"
        actions={
          <>
            <Button variant="secondary" onClick={() => navigate("/automations")}>Cancel</Button>
            <Button disabled={submitting || !activeWorkspaceCode || !name.trim() || !prompt.trim()} aria-busy={submitting} onClick={submitAutomation}>
              <Save size={15} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        }
      />
      <div className="max-w-5xl">
        <SurfaceCard className="space-y-5">
          <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_240px]">
            <Field label="Automation name">
              <TextInput aria-label="Automation name" placeholder="Daily workspace release digest" value={name} onChange={(event) => setName(event.target.value)} />
            </Field>
            <Field label="Automation status">
              <SelectInput aria-label="Automation status" value={status} onChange={(event) => setStatus(event.target.value as AutomationOperationalStatus)}>
                {editableAutomationStatuses.map((item) => (
                  <option key={item} value={item}>{item}</option>
                ))}
              </SelectInput>
            </Field>
          </div>
          <Field label="Description">
            <TextArea aria-label="Automation description" className="min-h-24" placeholder="Summarize the business intent, constraints, or expected operator-facing output." value={description} onChange={(event) => setDescription(event.target.value)} />
          </Field>
          <Field label="Cron expression">
            <TextInput aria-label="Cron expression" className="font-mono" placeholder="*/15 * * * *" value={cronExpression} onChange={(event) => setCronExpression(event.target.value)} />
          </Field>
          <div className="grid gap-1.5">
            <div className="flex items-center justify-between gap-3">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                Prompt body
              </span>
              <Button size="sm" type="button" variant="secondary" onClick={() => setPromptFullscreenOpen(true)}>
                <Maximize2 size={14} aria-hidden="true" />
                Fullscreen
              </Button>
            </div>
            <TextArea
              aria-label="Automation prompt"
              className="min-h-80 w-full"
              placeholder="Write the long-form prompt that the automation should dispatch for this workspace."
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
            />
          </div>
        </SurfaceCard>
      </div>
      {promptFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label="Prompt fullscreen editor"
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Automation prompt</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  Write the long-form prompt that will be dispatched whenever this automation runs.
                </p>
              </div>
              <Button size="icon" type="button" variant="ghost" aria-label="Close prompt fullscreen" onClick={() => setPromptFullscreenOpen(false)}>
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              <TextArea
                aria-label="Automation prompt fullscreen"
                className="h-full min-h-0 w-full resize-none"
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
              />
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
