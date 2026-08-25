import type { ReactNode } from "react";

export interface EditableTab {
  key: string;
  label: ReactNode;
  children: ReactNode;
  closable?: boolean;
}

export interface EditableTabEditInfo {
  key?: string;
  remainingTabs?: EditableTab[];
}

export interface EditableTabCloseRequest {
  id: number;
  key: string;
}

export interface EditableTabAddTriggerProps {
  onAdd: () => void;
}

export interface EditableTabsProps {
  items: readonly EditableTab[];
  closeTabRequest?: EditableTabCloseRequest | null;
  defaultActiveKey?: string;
  createTab?: () => EditableTab | undefined;
  hideAdd?: boolean;
  renderAddTrigger?: (props: EditableTabAddTriggerProps) => ReactNode;
  onChange?: (activeKey: string | undefined) => void;
  onEdit?: (action: OperateType, info: EditableTabEditInfo) => void;
  className?: string;
  keepMounted?: boolean;
}

export enum OperateType {
  Remove,
  Add,
}
