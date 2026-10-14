import { useToast } from "@/components/ui/toast";
import { SoulFields } from "./SoulFields";
import { getDudeSoul, saveDudeSoul, type SoulState } from "../api/dudeApi";
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
import { AgentLoopField } from "@/domain/agent/components/AgentLoopField";
import { AgentCandidateModelsField } from "@/domain/agent/components/AgentCandidateModelsField";
import { AgentRolePromptField } from "@/domain/agent/components/AgentRolePromptField";
import { AgentDescriptionField } from "@/domain/agent/components/AgentDescriptionField";
import {
  DudeResourceFields,
  type DudeResourceFieldsHandle,
} from "./DudeResourceFields";
import {
  createDude,
  updateDude,
  type Dude,
  type DudeInput,
} from "../api/dudeApi";

export function DudeEditorDialog({
  dude,
  onClose,
  onSaved,
}: {
  dude?: Dude;
  onClose: () => void;
  onSaved: () => Promise<unknown>;
}) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const soulBaseline = useRef<{ name: string; markdown: string } | null>(null);
  const reportError = (reason: unknown) => toast.error(reason instanceof Error ? reason.message : String(reason));
  const labelId = useId();
  const [section, setSection] = useState("profile");
  const [draft, setDraft] = useState<DudeInput>(() => ({
    dudeCode: dude?.dudeCode ?? "",
    name: dude?.name ?? "",
    description: dude?.description ?? "",
    rolePrompt: dude?.rolePrompt ?? "",
    loopCode: dude?.loopCode ?? "",
    avatarId: dude?.avatarId ?? "robot",
    candidateModelCodes: dude?.candidateModelCodes ?? [],
    enabled: dude?.enabled ?? true,
    heartbeatMinutes: dude?.heartbeatMinutes ?? 30,
    memoryAutoApprove: dude?.memoryAutoApprove ?? false,
    kind: dude?.kind === "Memory" ? "Memory" : "Custom",
  }));
  const [soulBody, setSoulBody] = useState("");
  const [soulState, setSoulState] = useState<SoulState>("Uninitialized");
  const [soulLoaded, setSoulLoaded] = useState(!dude);
  useEffect(() => {
    if (!dude) return;
    const controller = new AbortController();
    getDudeSoul(dude.dudeCode, controller.signal)
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
  }, [dude?.dudeCode]);
  const createdCode = useRef<string | null>(null);
  const resources = useRef<DudeResourceFieldsHandle>(null);
  const advancedResources = useRef<DudeResourceFieldsHandle>(null);
  const [saving, setSaving] = useState(false);
  const field = <K extends keyof DudeInput>(key: K, value: DudeInput[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));
  const valid =
    /^[a-z][a-z0-9_-]{0,63}$/.test(draft.dudeCode) &&
    Boolean(draft.name.trim()) &&
    [...draft.name].length <= 100 &&
    Boolean(draft.loopCode) &&
    Boolean(draft.rolePrompt.trim()) &&
    draft.heartbeatMinutes >= 1;
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
      if (dude || createdCode.current) {
        const { dudeCode: _code, kind: _kind, name: _name, ...body } = draft;
        const changes = dude
          ? Object.fromEntries(Object.entries(body).filter(([key, value]) =>
            JSON.stringify(value) !== JSON.stringify(dude[key as keyof Dude])))
          : body;
        if (Object.keys(changes).length) await updateDude(dude?.dudeCode ?? createdCode.current!, changes);
      } else {
        const created = await createDude(draft);
        createdCode.current = created.dudeCode;
      }
      if (!soulBaseline.current || soulBaseline.current.name !== draft.name || soulBaseline.current.markdown !== soulBody) {
        const saved = await saveDudeSoul(dude?.dudeCode ?? createdCode.current!, {
          name: draft.name, markdown: soulBody,
        });
        soulBaseline.current = { name: saved.name, markdown: saved.markdown };
      }
      await resources.current?.save();
      await advancedResources.current?.save();
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
        closeLabel={t("dude.close")}
      >
        <DialogHeader>
          <div className="flex items-center justify-between gap-4">
            <DialogTitle>{t(dude ? "dude.edit" : "dude.create")}</DialogTitle>
            {dude ? (
              <div className="flex items-center gap-2 pr-8 text-sm">
                <span>
                  {t(draft.enabled ? "dude.activated" : "dude.paused")}
                </span>
                <Switch
                  aria-label={t(draft.enabled ? "dude.pause" : "dude.resume")}
                  checked={draft.enabled}
                  disabled={saving}
                  onCheckedChange={(value) => field("enabled", value)}
                />
              </div>
            ) : null}
          </div>
          <DialogDescription>{t("dude.costWarning")}</DialogDescription>
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
              ...(dude ? ["resources", "advanced"] : []),
            ].map((value) => (
              <TabsTrigger key={value} value={value} className="flex-none px-3">
                {t(`dude.${value}Section`)}
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
                  label={t("dude.code")}
                  required
                  hint={t("dude.codeHelp")}
                >
                  <TextInput
                    required
                    aria-label={t("dude.code")}
                    maxLength={64}
                    disabled={
                      saving || Boolean(dude) || Boolean(createdCode.current)
                    }
                    value={draft.dudeCode}
                    onChange={(event) => field("dudeCode", event.target.value)}
                  />
                </Field>
                <AgentDescriptionField value={draft.description ?? ""} disabled={saving} onChange={(value) => field("description", value)} />
                <AgentLoopField
                  value={draft.loopCode}
                  defaultKind={!dude ? "React" : undefined}
                  disabled={saving}
                  onChange={(value) => field("loopCode", value)}
                />
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
              {dude ? (
                <TabsContent
                  value="resources"
                  keepMounted
                  className="space-y-4 py-2"
                >
                  <DudeResourceFields
                    ref={resources}
                    dudeCode={dude.dudeCode}
                    panel="resources"
                    disabled={saving}
                  />
                </TabsContent>
              ) : null}
              {dude ? (
                <TabsContent
                  value="advanced"
                  keepMounted
                  className="space-y-4 py-2"
                >
                  <DudeResourceFields
                    ref={advancedResources}
                    dudeCode={dude.dudeCode}
                    panel="advanced"
                    disabled={saving}
                  />
                </TabsContent>
              ) : null}
              <TabsContent
                value="background"
                keepMounted
                className="space-y-5 py-2"
              >
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span id={`${labelId}-background`}>
                    {t("dude.backgroundEnabled")}
                  </span>
                  <Switch
                    aria-labelledby={`${labelId}-background`}
                    checked={draft.enabled}
                    disabled={saving}
                    onCheckedChange={(value) => field("enabled", value)}
                  />
                </div>
                <Field label={t("dude.heartbeatMinutes")}>
                  <TextInput
                    type="number"
                    min={1}
                    max={10080}
                    aria-label={t("dude.heartbeatMinutes")}
                    value={draft.heartbeatMinutes}
                    disabled={saving}
                    onChange={(event) =>
                      field("heartbeatMinutes", Number(event.target.value))
                    }
                  />
                </Field>
                {draft.kind === "Memory" ? (
                  <div className="flex items-start justify-between gap-3 text-sm">
                    <span>
                      <span id={`${labelId}-auto-approve`}>
                        {t("dude.autoApprove")}
                      </span>
                      <span
                        id={`${labelId}-auto-approve-help`}
                        className="mt-1 block text-xs text-[var(--color-on-surface-variant)]"
                      >
                        {t("dude.autoApproveHelp")}
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
                {t("dude.cancel")}
              </Button>
              <Button type="submit" disabled={saving || !valid}>
                {t(saving ? "dude.saving" : "dude.save")}
              </Button>
            </DialogFooter>
          </form>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
