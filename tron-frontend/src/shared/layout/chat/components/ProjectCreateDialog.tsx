import { X, Folder } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppOptions } from "@/domain/app";
import { createProject } from "@/domain/project";
import type { ProjectRecord } from "@/domain/project";
import { useToast } from "@/components/ui/toast";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

export function ProjectCreateDialog({
  onSaved,
  onOpenChange,
  open,
  project,
  workspaceCode,
}: {
  onSaved: (project: ProjectRecord) => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  project?: ProjectRecord;
  workspaceCode: string;
}) {
  const { options, loading, error, refresh } = useAppOptions({ workspaceCode });
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [name, setName] = useState("");
  const [appCodes, setAppCodes] = useState<string[]>([]);
  const [primaryAppCode, setPrimaryAppCode] = useState<string>();
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    if (!open) return;
    refresh();
    setName(project?.name ?? "");
    setAppCodes(project?.appCodes ?? []);
    setPrimaryAppCode(project?.primaryAppCode ?? undefined);
  }, [open, project, refresh]);
  function toggleApp(appCode: string) {
    setAppCodes((current) => {
      const next = current.includes(appCode)
        ? current.filter((code) => code !== appCode)
        : [...current, appCode];
      setPrimaryAppCode((primary) =>
        next.includes(primary ?? "") ? primary : next[0],
      );
      return next;
    });
  }
  const selectedApps = useMemo(() => {
    const appsByCode = new Map(options.map((app) => [app.appCode, app]));
    return appCodes.map((appCode) => ({
      appCode,
      name: appsByCode.get(appCode)?.name ?? appCode,
    }));
  }, [appCodes, options]);
  const resolvedPrimaryAppCode = appCodes.includes(primaryAppCode ?? "")
    ? primaryAppCode
    : appCodes[0] ?? null;
  async function submit() {
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      const savedProject = await createProject({
        projectCode: project?.projectCode,
        workspaceCode,
        name,
        appCodes,
        primaryAppCode: resolvedPrimaryAppCode,
      });
      onSaved(savedProject);
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
        <div className="flex flex-col gap-6">
          <Field>
            <FieldLabel htmlFor="project-name">{t("shell:chatSidebar.projectDialog.nameLabel")}</FieldLabel>
            <Input
              id="project-name"
              placeholder={t("shell:chatSidebar.projectDialog.namePlaceholder")}
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </Field>
          <Field>
            <FieldLabel>{t("shell:chatSidebar.projectDialog.linkedApps")}</FieldLabel>
            <FieldDescription>
              {t("shell:chatSidebar.projectDialog.linkedAppsDescription")}
            </FieldDescription>
            <div className="w-full overflow-hidden rounded-[4px] border border-input">
              {selectedApps.map((app) => (
                <div
                  className="flex h-8 items-center gap-2 border-b border-border px-3 last:border-b-0"
                  key={app.appCode}
                >
                  <Folder
                    aria-hidden="true"
                    className="size-4 shrink-0 text-muted-foreground"
                  />
                  <span className="min-w-0 flex-1 truncate">{app.name}</span>
                  {selectedApps.length > 1 ? (
                    app.appCode === resolvedPrimaryAppCode ? (
                      <Badge>{t("shell:chatSidebar.projectDialog.primary")}</Badge>
                    ) : (
                      <Badge
                        variant="outline"
                        onClick={() => setPrimaryAppCode(app.appCode)}
                      >
                        {t("shell:chatSidebar.projectDialog.setPrimary")}
                      </Badge>
                    )
                  ) : null}
                  <Button
                    className="shrink-0 w-auto"
                    size="icon"
                    variant="link"
                    onClick={() => toggleApp(app.appCode)}
                  >
                    <X size={16} />
                  </Button>
                </div>
              ))}
              <DropdownMenu onOpenChange={(menuOpen) => { if (menuOpen) refresh(); }}>
                <DropdownMenuTrigger
                  render={
                    <Button
                      className="w-full h-8 justify-start gap-2 rounded-none px-3 no-underline hover:no-underline"
                      type="button"
                      variant="link"
                    >

                      <span className="flex-1 text-left">{t("shell:chatSidebar.projectDialog.addApp")}</span>

                    </Button>
                  }
                />
                <DropdownMenuContent
                  align="start"
                  className="min-w-64 duration-10"
                >
                  {loading ? (
                    <DropdownMenuItem disabled>{t("shell:chatSidebar.projectDialog.loadingApps")}</DropdownMenuItem>
                  ) : error ? (
                    <DropdownMenuItem onClick={refresh}>{t("shell:chatSidebar.projectDialog.retryApps")}</DropdownMenuItem>
                  ) : options.length === 0 ? (
                    <DropdownMenuItem disabled>{t("shell:chatSidebar.projectDialog.noApps")}</DropdownMenuItem>
                  ) : options.map((app) => (
                    <DropdownMenuCheckboxItem
                      checked={appCodes.includes(app.appCode)}
                      key={app.appCode}
                      closeOnClick={false}
                      onCheckedChange={() => toggleApp(app.appCode)}
                    >
                      {app.name}
                    </DropdownMenuCheckboxItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </Field>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("shell:chatSidebar.project.cancel")}
          </Button>
          <Button
            disabled={!name.trim() || saving}
            onClick={() => void submit()}
          >
            {saving
              ? t("shell:chatSidebar.projectDialog.saving")
              : project
                ? t("shell:chatSidebar.projectDialog.saveChanges")
                : t("shell:chatSidebar.projectDialog.createTitle")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
