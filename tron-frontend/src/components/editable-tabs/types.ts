import type { ReactNode } from "react";

export interface EditableTab {
  key: string;
  label: ReactNode;
  children: ReactNode;
  closable?: boolean;
}

export type EditableTabAction = "add" | "remove";

export interface EditableTabEditInfo {
  key?: string;
}

export interface EditableTabAddTriggerProps {
  onAdd: () => void;
}

export interface EditableTabsProps {
  items: readonly EditableTab[];
  defaultActiveKey?: string;
  createTab?: () => EditableTab | undefined;
  hideAdd?: boolean;
  renderAddTrigger?: (props: EditableTabAddTriggerProps) => ReactNode;
  onChange?: (activeKey: string | undefined) => void;
  onEdit?: (action: EditableTabAction, info: EditableTabEditInfo) => void;
  className?: string;
  keepMounted?: boolean;
}
