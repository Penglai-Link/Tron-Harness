import { useEffect, useState } from "react";
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
        next.includes(primary ?? "") ? primary : next[0]
      );
      return next;
    });
  }
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
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{project ? "编辑项目" : "创建项目"}</DialogTitle>
        </DialogHeader>
        <label className="grid gap-1.5 text-sm">
          项目名称
          <input
            className="h-9 rounded border border-input bg-background px-2"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <fieldset className="grid gap-2">
          <legend className="text-sm font-medium">关联应用</legend>
          <p className="text-xs text-muted-foreground">
            首个选择的应用默认作为目录。可通过单选切换主应用，项目中的全部应用目录均可供会话访问。
          </p>
          {options.map((app) => (
            <div className="flex items-center gap-2 text-sm" key={app.appCode}>
              <input
                checked={appCodes.includes(app.appCode)}
                type="checkbox"
                onChange={() => toggleApp(app.appCode)}
              />
              <span className="min-w-0 flex-1 truncate">{app.name}</span>
              <label className="flex items-center gap-1 text-xs text-muted-foreground">
                <input
                  checked={primaryAppCode === app.appCode}
                  disabled={!appCodes.includes(app.appCode)}
                  name="project-primary-app"
                  type="radio"
                  onChange={() => setPrimaryAppCode(app.appCode)}
                />
                主目录
              </label>
            </div>
          ))}
        </fieldset>
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
