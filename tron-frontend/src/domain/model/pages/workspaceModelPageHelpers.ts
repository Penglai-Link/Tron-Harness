import { formatModelProtocol } from "@/domain/model/constants/modelPageOptions";
import type {
  ModelRecord,
  ModelSelectOption,
} from "@/domain/model/model";

export function isWorkspaceOwnedRecord(record: ModelRecord | null, workspaceCode: string) {
  return Boolean(
    record
    && record.workspaceCode === workspaceCode
    && (record.sourceKind === "LOCAL" || record.sourceKind === "OVERRIDE"),
  );
}

export function isEditableWorkspaceLocalRecord(record: ModelRecord | null, workspaceCode: string) {
  return Boolean(
    record
    && record.workspaceCode === workspaceCode
    && record.sourceKind === "LOCAL",
  );
}

export function isEditableWorkspaceDefinitionRecord(record: ModelRecord | null, workspaceCode: string) {
  return Boolean(
    record
    && record.workspaceCode === workspaceCode
    && record.sourceKind !== "PLATFORM",
  );
}

export function formatWorkspaceSelectorLabel(option: ModelSelectOption) {
  return `${option.displayName} - ${option.modelName} · ${option.sourceLabel} · ${option.vendor}/${formatModelProtocol(option.protocol, option.modelType)}`;
}
