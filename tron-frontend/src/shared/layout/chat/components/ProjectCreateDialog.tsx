import { ChevronDown, Folder, FolderPlus, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
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
  const { options } = useAppOptions({ workspaceCode });
  const toast = useToast();
  const [name, setName] = useState("");
  const [appCodes, setAppCodes] = useState<string[]>([]);
  const [primaryAppCode, setPrimaryAppCode] = useState<string>();
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    if (!open) return;
    setName(project?.name ?? "");
    setAppCodes(project?.appCodes ?? []);
    setPrimaryAppCode(project?.primaryAppCode);
  }, [open, project]);
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
    : appCodes[0];
  async function submit() {
    if (!name.trim() || appCodes.length === 0 || saving) return;
    setSaving(true);
    try {
      const savedProject = await createProject({
        projectCode: project?.projectCode,
        workspaceCode,
        name,
        appCodes,
        primaryAppCode: primaryAppCode ?? appCodes[0],
      });
      onSaved(savedProject);
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "保存项目失败");
    } finally {
      setSaving(false);
    }
  }
  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{project ? "编辑项目" : "创建项目"}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-6">
          <Field>
            <FieldLabel htmlFor="project-name">项目名称</FieldLabel>
            <Input
              id="project-name"
              placeholder="输入项目名称"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </Field>
          <Field>
            <FieldLabel>关联应用</FieldLabel>
            <FieldDescription>
              选择项目会话可以访问的应用目录。
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
                      <Badge>主要</Badge>
                    ) : (
                      <Badge
                        variant="outline"
                        onClick={() => setPrimaryAppCode(app.appCode)}
                      >
                        设为主要
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
              <DropdownMenu>
                <DropdownMenuTrigger
                  render={
                    <Button
                      className="w-full h-8 justify-start gap-2 rounded-none px-3 no-underline hover:no-underline"
                      type="button"
                      variant="link"
                    >
                      <FolderPlus
                        aria-hidden="true"
                        className="size-4 text-muted-foreground"
                      />
                      <span className="flex-1 text-left">添加应用</span>
                      <ChevronDown
                        aria-hidden="true"
                        className="size-4 text-muted-foreground"
                      />
                    </Button>
                  }
                />
                <DropdownMenuContent
                  align="start"
                  className="min-w-64 duration-10"
                >
                  {options.map((app) => (
                    <DropdownMenuCheckboxItem
                      checked={appCodes.includes(app.appCode)}
                      key={app.appCode}
                      onSelect={(event) => event.preventDefault()}
                      onCheckedChange={() => toggleApp(app.appCode)}
                    >
                      {app.name}
                    </DropdownMenuCheckboxItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
            {selectedApps.length === 0 ? (
              <FieldDescription className="text-destructive">
                请选择至少一个应用
              </FieldDescription>
            ) : null}
          </Field>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            disabled={!name.trim() || appCodes.length === 0 || saving}
            onClick={() => void submit()}
          >
            {saving ? "保存中" : project ? "保存修改" : "创建项目"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
