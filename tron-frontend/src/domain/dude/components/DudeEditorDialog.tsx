import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
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
  const labelId = useId();
  const [section, setSection] = useState("profile");
  const [draft, setDraft] = useState<DudeInput>(() => ({
    dudeCode: dude?.dudeCode ?? "",
    name: dude?.name ?? "",
    rolePrompt: dude?.rolePrompt ?? "",
    loopCode: dude?.loopCode ?? "",
    avatarId: dude?.avatarId ?? "robot",
    candidateModelCodes: dude?.candidateModelCodes ?? [],
    enabled: dude?.enabled ?? true,
    heartbeatMinutes: dude?.heartbeatMinutes ?? 30,
    memoryAutoApprove: dude?.memoryAutoApprove ?? false,
    kind: dude?.kind === "Memory" ? "Memory" : "Custom",
  }));
  const createdCode = useRef<string | null>(null);
  const resources = useRef<DudeResourceFieldsHandle>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const field = <K extends keyof DudeInput>(key: K, value: DudeInput[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));
  const valid =
    /^[a-z][a-z0-9_-]{0,63}$/.test(draft.dudeCode) &&
    Boolean(draft.name.trim()) &&
    Boolean(draft.loopCode) &&
    draft.heartbeatMinutes >= 1;
  async function save() {
    if (!valid || saving) return;
    setSaving(true);
    setError(null);
    try {
      if (dude || createdCode.current) {
        const { dudeCode: _code, kind: _kind, ...body } = draft;
        await updateDude(dude?.dudeCode ?? createdCode.current!, body);
      } else {
        const created = await createDude(draft);
        createdCode.current = created.dudeCode;
      }
      await resources.current?.save();
      await onSaved();
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
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
        className="flex max-h-[90dvh] flex-col overflow-hidden sm:max-w-2xl"
        closeLabel={t("dude.close")}
      >
        <DialogHeader>
          <DialogTitle>{t(dude ? "dude.edit" : "dude.create")}</DialogTitle>
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
            {["profile", "background", ...(dude ? ["resources"] : [])].map(
              (value) => (
                <TabsTrigger
                  key={value}
                  value={value}
                  className="flex-none px-3"
                >
                  {t(`dude.${value}Section`)}
                </TabsTrigger>
              )
            )}
          </TabsList>
          <form
            className="flex min-h-0 flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <div className="min-h-0 overflow-y-auto px-1">
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
                <Field label={t("dude.name")} required>
                  <TextInput
                    required
                    aria-label={t("dude.name")}
                    disabled={saving}
                    value={draft.name}
                    onChange={(event) => field("name", event.target.value)}
                  />
                </Field>
                {!dude ? (
                  <Field label={t("dude.kind")}>
                    <SelectInput
                      aria-label={t("dude.kind")}
                      value={draft.kind}
                      disabled={saving || Boolean(createdCode.current)}
                      onValueChange={(value) => {
                        if (value === "Custom" || value === "Memory")
                          field("kind", value);
                      }}
                    >
                      <SelectItem value="Custom">{t("dude.custom")}</SelectItem>
                      <SelectItem value="Memory">{t("dude.memory")}</SelectItem>
                    </SelectInput>
                  </Field>
                ) : null}
                <AgentLoopField
                  value={draft.loopCode}
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
                  value={draft.rolePrompt}
                  disabled={saving}
                  onChange={(value) => field("rolePrompt", value)}
                />
              </TabsContent>
              {dude ? (
                <TabsContent value="resources" keepMounted>
                  <DudeResourceFields
                    ref={resources}
                    dudeCode={dude.dudeCode}
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
            {error ? (
              <p role="alert" className="text-sm text-[var(--color-error)]">
                {error}
              </p>
            ) : null}
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
