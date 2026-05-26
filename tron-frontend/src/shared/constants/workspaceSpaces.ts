import type {
  PrimarySpace,
  SecondarySpace,
  WorkspaceOption,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";

export const defaultPrimarySpaceId = "ENTERPRISE";
export const defaultSecondarySpaceId = "TRON_CORE";

export function getEmptyWorkspaceSwitcher(): WorkspaceSwitcherData {
  return {
    primarySpaces: [],
    secondarySpaces: [],
  };
}

export const workspaceSpaceApiContracts = {
  listWorkspaceOptions: "GET /api/v1/admin/workspaces/options",
  getWorkspaceSwitcher: "GET /api/v1/admin/workspaces/switcher",
} as const;

export const primarySpaceMocks: PrimarySpace[] = [
  {
    workspaceCode: "ENTERPRISE",
    name: "Enterprise",
    description: "Global governance and shared AI resource policy.",
    childCount: 1,
    status: "Active",
  },
  {
    workspaceCode: "COMPLIANCE",
    name: "Compliance",
    description: "Audit, retention, permission, and regulated resource controls.",
    childCount: 0,
    status: "Active",
  },
];

export const secondarySpaceMocks: SecondarySpace[] = [
  {
    workspaceCode: "ENTERPRISE",
    primaryWorkspaceCode: "ENTERPRISE",
    parentWorkspaceCode: null,
    label: "Enterprise",
    name: "Enterprise",
    description: "Global governance and shared AI resource policy.",
    status: "Active",
    depth: 1,
    childCount: 1,
    effectiveUserCount: 1,
  },
  {
    workspaceCode: "PLATFORM",
    primaryWorkspaceCode: "ENTERPRISE",
    parentWorkspaceCode: "ENTERPRISE",
    label: "Enterprise / Platform",
    name: "Platform",
    description: "Agent runtime governance and admin operations.",
    status: "Active",
    depth: 2,
    childCount: 1,
    effectiveUserCount: 3,
  },
  {
    workspaceCode: "TRON_CORE",
    primaryWorkspaceCode: "ENTERPRISE",
    parentWorkspaceCode: "PLATFORM",
    label: "Enterprise / Platform / Tron Core",
    name: "Tron Core",
    description: "Main Tron console workspace.",
    status: "Active",
    depth: 3,
    childCount: 0,
    effectiveUserCount: 2,
  },
  {
    workspaceCode: "COMPLIANCE",
    primaryWorkspaceCode: "COMPLIANCE",
    parentWorkspaceCode: null,
    label: "Compliance",
    name: "Compliance",
    description: "Audit, retention, permission, and regulated resource controls.",
    status: "Active",
    depth: 1,
    childCount: 0,
    effectiveUserCount: 1,
  },
];

export const workspaceOptionMocks: WorkspaceOption[] = [
  {
    workspaceCode: "ENTERPRISE",
    label: "Enterprise",
    name: "Enterprise",
    description: "Global governance and shared AI resource policy.",
    parentWorkspaceCode: null,
    depth: 1,
    status: "Active",
  },
  {
    workspaceCode: "PLATFORM",
    label: "Enterprise / Platform",
    name: "Platform",
    description: "Agent runtime governance and admin operations.",
    parentWorkspaceCode: "ENTERPRISE",
    depth: 2,
    status: "Active",
  },
  {
    workspaceCode: "TRON_CORE",
    label: "Enterprise / Platform / Tron Core",
    name: "Tron Core",
    description: "Main Tron console workspace.",
    parentWorkspaceCode: "PLATFORM",
    depth: 3,
    status: "Active",
  },
  {
    workspaceCode: "COMPLIANCE",
    label: "Compliance",
    name: "Compliance",
    description: "Audit, retention, permission, and regulated resource controls.",
    parentWorkspaceCode: null,
    depth: 1,
    status: "Active",
  },
];

export function getSecondarySpaceMocks(primaryWorkspaceCode: PrimarySpace["workspaceCode"]) {
  return secondarySpaceMocks.filter((space) => space.primaryWorkspaceCode === primaryWorkspaceCode);
}

export function getWorkspaceOptionMocks() {
  return workspaceOptionMocks;
}

export function getWorkspaceSwitcherMock(): WorkspaceSwitcherData {
  return {
    primarySpaces: primarySpaceMocks,
    secondarySpaces: secondarySpaceMocks,
  };
}
