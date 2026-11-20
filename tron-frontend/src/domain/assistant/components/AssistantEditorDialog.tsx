import { useToast } from "@/components/ui/toast";
import { SoulFields } from "./SoulFields";
import { getAssistantSoul, saveAssistantSoul, type SoulState } from "../api/assistantApi";
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AgentAvatarPicker } from "@/domain/agent/components/AgentAvatarPicker";
import { AgentCandidateModelsField } from "@/domain/agent/components/AgentCandidateModelsField";
import { AgentRolePromptField } from "@/domain/agent/components/AgentRolePromptField";
import { AgentDescriptionField } from "@/domain/agent/components/AgentDescriptionField";
import {
  AgentResourcesEditor,
  type AgentResourcesEditorHandle,
} from "@/domain/agent/components/AgentResourcesEditor";
import {
  createAssistant,
  updateAssistant,
  getAssistantResources,
  type Assistant,
  type AssistantInput,
} from "../api/assistantApi";

export function AssistantEditorDialog({
  assistant,
  onClose,
  onSaved,
}: {
  assistant?: Assistant;
  onClose: () => void;
  onSaved: () => Promise<unknown>;
}) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const soulBaseline = useRef<{ name: string; markdown: string } | null>(null);
  const reportError = (reason: unknown) => toast.error(reason instanceof Error ? reason.message : String(reason));
  const labelId = useId();
  const [section, setSection] = useState("profile");
  const [draft, setDraft] = useState<AssistantInput>(() => ({
    assistantCode: assistant?.assistantCode ?? "",
    name: assistant?.name ?? "",
    description: assistant?.description ?? "",
    rolePrompt: assistant?.rolePrompt ?? "",

    avatarId: assistant?.avatarId ?? "robot",
    candidateModelCodes: assistant?.candidateModelCodes ?? [],
    enabled: assistant?.enabled ?? true,
    heartbeatMinutes: assistant?.heartbeatMinutes ?? 10,
    memoryAutoApprove: assistant?.memoryAutoApprove ?? false,
    kind: assistant?.kind === "Memory" ? "Memory" : "Custom",
  }));
  const [soulBody, setSoulBody] = useState("");
  const [soulState, setSoulState] = useState<SoulState>("Uninitialized");
  const [soulLoaded, setSoulLoaded] = useState(!assistant);
  useEffect(() => {
    if (!assistant) return;
    const controller = new AbortController();
    getAssistantSoul(assistant.assistantCode, controller.signal)
      .then((soul) => {
        soulBaseline.current = { name: soul.name, markdown: soul.markdown };
        setSoulBody(soul.markdown);
        setSoulState(soul.state);
        field("name", soul.name);
        setSoulLoaded(true);
      })
      .catch((reason) => {
        if (!controller.signal.aborted) reportError(reason);
      });
    return () => controller.abort();
  }, [assistant?.assistantCode]);
  const createdCode = useRef<string | null>(null);
  const resources = useRef<AgentResourcesEditorHandle>(null);
  const [saving, setSaving] = useState(false);
  const field = <K extends keyof AssistantInput>(key: K, value: AssistantInput[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));
  const valid =
    /^[a-z][a-z0-9_-]{0,63}$/.test(draft.assistantCode) &&
    Boolean(draft.name.trim()) &&
    [...draft.name].length <= 100 &&
    Boolean(draft.rolePrompt.trim()) &&
    Number.isInteger(draft.heartbeatMinutes) && draft.heartbeatMinutes >= 1 && draft.heartbeatMinutes <= 10080;
  async function save() {
    if (
      !valid ||
      saving ||
      !soulLoaded ||
      new TextEncoder().encode(soulBody).length > 16384
    )
      return;
    setSaving(true);
    try {
      resources.current?.validate();
      if (assistant || createdCode.current) {
        const { assistantCode: _code, kind: _kind, name: _name, ...body } = draft;
        const changes = assistant
          ? Object.fromEntries(Object.entries(body).filter(([key, value]) =>
            JSON.stringify(value) !== JSON.stringify(assistant[key as keyof Assistant])))
          : body;
        if (Object.keys(changes).length) await updateAssistant(assistant?.assistantCode ?? createdCode.current!, changes);
      } else {
        const created = await createAssistant(draft);
        createdCode.current = created.assistantCode;
      }
      if (!soulBaseline.current || soulBaseline.current.name !== draft.name || soulBaseline.current.markdown !== soulBody) {
        const saved = await saveAssistantSoul(assistant?.assistantCode ?? createdCode.current!, {
          name: draft.name, markdown: soulBody,
        });
        soulBaseline.current = { name: saved.name, markdown: saved.markdown };
      }
      await resources.current?.save(assistant?.assistantCode ?? createdCode.current!);
      await onSaved();
      onClose();
    } catch (reason) {
      reportError(reason);
    } finally {
      setSaving(false);
    }
  }
  return (
    <Dialog
      open
      disablePointerDismissal
      onOpenChange={(open, details) => {
        if (saving) details.cancel();
        else if (!open) onClose();
      }}
    >
      <DialogContent
        className="flex h-[min(78dvh,48rem)] max-h-[90dvh] flex-col overflow-hidden sm:max-w-2xl"
        closeLabel={t("assistant.close")}
      >
        <DialogHeader>
          <div className="flex items-center justify-between gap-4">
            <DialogTitle>{t(assistant ? "assistant.edit" : "assistant.create")}</DialogTitle>
            <div className="flex items-center gap-2 pr-8 text-sm">
                <span>
                  {t(draft.enabled ? "assistant.activated" : "assistant.paused")}
                </span>
                <Switch
                  aria-label={t(draft.enabled ? "assistant.pause" : "assistant.resume")}
                  checked={draft.enabled}
                  disabled={saving}
                  onCheckedChange={(value) => field("enabled", value)}
                />
              </div>
          </div>
          <DialogDescription>{t("assistant.costWarning")}</DialogDescription>
        </DialogHeader>
        <Tabs
          className="min-h-0 flex-1"
          value={section}
          onValueChange={(value) => {
            if (typeof value === "string") setSection(value);
          }}
        >
          <TabsList
            variant="line"
            className="w-full justify-start border-b border-[var(--color-outline-variant)]"
          >
            {[
              "profile",
              "soul",
              "background",
              "resources", "advanced",
            ].map((value) => (
              <TabsTrigger key={value} value={value} className="flex-none px-3">
                {t(`assistant.${value}Section`)}
              </TabsTrigger>
            ))}
          </TabsList>
          <form
            className="flex min-h-0 flex-1 flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <div className="min-h-0 flex-1 overflow-y-auto px-1">
              <TabsContent
                value="profile"
                keepMounted
                className="space-y-4 pb-2"
              >
                <AgentAvatarPicker
                  value={draft.avatarId}
                  disabled={saving}
                  onChange={(value) => field("avatarId", value)}
                />
                <Field
                  label={t("assistant.code")}
                  required
                  hint={t("assistant.codeHelp")}
                >
                  <TextInput
                    required
                    aria-label={t("assistant.code")}
                    maxLength={64}
                    disabled={
                      saving || Boolean(assistant) || Boolean(createdCode.current)
                    }
                    value={draft.assistantCode}
                    onChange={(event) => field("assistantCode", event.target.value)}
                  />
                </Field>
                <AgentDescriptionField value={draft.description ?? ""} disabled={saving} onChange={(value) => field("description", value)} />

                <AgentCandidateModelsField
                  workspaceCode="$platform"
                  value={draft.candidateModelCodes}
                  disabled={saving}
                  onChange={(value) => field("candidateModelCodes", value)}
                />
                <AgentRolePromptField
                  rows={8}
                  required
                  value={draft.rolePrompt}
                  disabled={saving}
                  onChange={(value) => field("rolePrompt", value)}
                />
              </TabsContent>
              <TabsContent value="soul" keepMounted className="space-y-4 py-2">
                <SoulFields
                  name={draft.name}
                  markdown={soulBody}
                  state={soulState}
                  disabled={saving || !soulLoaded}
                  onName={(v) => {
                    field("name", v);
                  }}
                  onMarkdown={(v) => {
                    setSoulBody(v);
                  }}
                  onSave={() => void save()}
                />
              </TabsContent>
              <AgentResourcesEditor ref={resources} agentCode={assistant?.assistantCode} agentName={draft.name} loadRecord={getAssistantResources} disabled={saving} audience="assistant" />
              <TabsContent
                value="background"
                keepMounted
                className="space-y-5 py-2"
              >
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span id={`${labelId}-background`}>
                    {t("assistant.backgroundEnabled")}
                  </span>
                  <Switch
                    aria-labelledby={`${labelId}-background`}
                    checked={draft.enabled}
                    disabled={saving}
                    onCheckedChange={(value) => field("enabled", value)}
                  />
                </div>
                <Field label={t("assistant.heartbeatMinutes")}>
                  <TextInput
                    type="text"
                    inputMode="numeric"
                    min={1}
                    max={10080}
                    aria-label={t("assistant.heartbeatMinutes")}
                    value={Number.isFinite(draft.heartbeatMinutes) ? draft.heartbeatMinutes : ""}
                    disabled={saving}
                    onChange={(event) =>
                      /^\d*$/.test(event.target.value) && field("heartbeatMinutes", event.target.value === "" ? NaN : Number(event.target.value))
                    }
                  />
                </Field>
                {draft.kind === "Memory" ? (
                  <div className="flex items-start justify-between gap-3 text-sm">
                    <span>
                      <span id={`${labelId}-auto-approve`}>
                        {t("assistant.autoApprove")}
                      </span>
                      <span
                        id={`${labelId}-auto-approve-help`}
                        className="mt-1 block text-xs text-[var(--color-on-surface-variant)]"
                      >
                        {t("assistant.autoApproveHelp")}
                      </span>
                    </span>
                    <Switch
                      aria-labelledby={`${labelId}-auto-approve`}
                      aria-describedby={`${labelId}-auto-approve-help`}
                      checked={draft.memoryAutoApprove}
                      disabled={saving}
                      onCheckedChange={(value) =>
                        field("memoryAutoApprove", value)
                      }
                    />
                  </div>
                ) : null}
              </TabsContent>
            </div>
            <DialogFooter className="shrink-0 border-t border-[var(--color-outline-variant)] pt-4">
              <Button
                type="button"
                variant="outline"
                disabled={saving}
                onClick={onClose}
              >
                {t("assistant.cancel")}
              </Button>
              <Button type="submit" disabled={saving || !valid}>
                {t(saving ? "assistant.saving" : "assistant.save")}
              </Button>
            </DialogFooter>
          </form>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
