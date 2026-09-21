export * from "./StatusTone";
export * from "./permissionDomains";
export const adminAuditStatusOptions = [
  { label: "All status", value: "all" },
  { label: "Success", value: "Success" },
  { label: "Warning", value: "Warning" },
  { label: "Failed", value: "Failed" },
] as const;
