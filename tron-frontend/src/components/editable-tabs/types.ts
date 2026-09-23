import type { ReactNode } from "react";

export interface EditableTab {
  key: string;
  stableId?: string;
  ownerKey?: string;
  hidden?: boolean;
  url?: string;
  label: ReactNode;
  /** 覆盖 label 文本的截断样式（用于控制最大宽度等）。 */
  labelClassName?: string;
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
  /** 受控激活的 tab key；传入后以该值同步内部激活态 */
  activeKey?: string;
  defaultActiveKey?: string;
  createTab?: () => EditableTab | undefined;
  hideAdd?: boolean;
  renderAddTrigger?: (props: EditableTabAddTriggerProps) => ReactNode;
  renderTrailingAction?: () => ReactNode;
  onChange?: (activeKey: string | undefined) => void;
  onEdit?: (action: OperateType, info: EditableTabEditInfo) => void;
  className?: string;
  keepMounted?: boolean;
  /**
   * Keep tabs created only by this component when the caller has not added
   * them to `items`. Controlled tab groups should disable this so a removed
   * external tab is unmounted immediately.
   */
  preserveInternalTabs?: boolean;
  /** 为 Tab 子内容提供当前可见性上下文，适用于需要同步外部视图显隐的内容。 */
  provideTabVisibility?: boolean;
}

export enum OperateType {
  Remove,
  Add,
}
