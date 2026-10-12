import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";

const defaultName = "默认工作空间";
const defaultDescription = "PersonAI 档案的默认工作空间";

/** Translate the seeded default workspace without changing user-defined names. */
export function localizeWorkspaceOptions(options: WorkspaceOption[], translatedDefaultName: string): WorkspaceOption[] {
  const byCode = new Map(options.map((option) => [option.workspaceCode, option]));
  const isDefault = (option: WorkspaceOption) => !option.parentWorkspaceCode && option.name === defaultName && option.description === defaultDescription;
  return options.map((option) => {
    let ancestor: WorkspaceOption | undefined = option;
    const visited = new Set<string>();
    while (ancestor && !visited.has(ancestor.workspaceCode)) {
      visited.add(ancestor.workspaceCode);
      if (isDefault(ancestor)) {
        const label = option.label === defaultName || option.label.startsWith(`${defaultName} / `)
          ? translatedDefaultName + option.label.slice(defaultName.length)
          : option.label;
        return { ...option, name: isDefault(option) ? translatedDefaultName : option.name, label };
      }
      ancestor = ancestor.parentWorkspaceCode ? byCode.get(ancestor.parentWorkspaceCode) : undefined;
    }
    return option;
  });
}
