import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { createProject } from "@/domain/project";
import type { ProjectRecord } from "@/domain/project";
import { useToast } from "@/components/ui/toast";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";

export function ProjectCreateDialog({ onSaved, onOpenChange, open, project, workspaceCode }: {
  onSaved: (project: ProjectRecord) => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  project?: ProjectRecord;
  workspaceCode: string;
}) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) setName(project?.name ?? "");
  }, [open, project]);

  async function submit() {
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      const saved = await createProject({
        projectCode: project?.projectCode,
        workspaceCode,
        name: name.trim(),
        appCodes: project?.appCodes ?? [],
        primaryAppCode: project?.primaryAppCode ?? null,
      });
      onSaved(saved);
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.project.saveFailed"));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{project ? t("shell:chatSidebar.projectDialog.editTitle") : t("shell:chatSidebar.projectDialog.createTitle")}</DialogTitle>
        </DialogHeader>
        <Field>
          <FieldLabel className="gap-1" htmlFor="project-name">
            {t("shell:chatSidebar.projectDialog.nameLabel")}
            <span className="text-destructive-foreground" aria-hidden="true">*</span>
          </FieldLabel>
          <Input id="project-name" required placeholder={t("shell:chatSidebar.projectDialog.namePlaceholder")}
            value={name} onChange={(event) => setName(event.target.value)} />
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{t("shell:chatSidebar.project.cancel")}</Button>
          <Button disabled={!name.trim() || saving} onClick={() => void submit()}>
            {saving ? t("shell:chatSidebar.projectDialog.saving") : project
              ? t("shell:chatSidebar.projectDialog.saveChanges") : t("shell:chatSidebar.projectDialog.createTitle")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
