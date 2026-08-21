const AVATAR_RESOURCE_PREFIX = "resource://";

export interface AvatarResourceReference {
  workspaceCode: string;
  resourceCode: string;
}

export function parseAvatarResourceReference(value: string | null | undefined): AvatarResourceReference | null {
  if (!value?.startsWith(AVATAR_RESOURCE_PREFIX)) return null;

  const [encodedWorkspaceCode, encodedResourceCode] = value.slice(AVATAR_RESOURCE_PREFIX.length).split("/", 2);
  if (!encodedWorkspaceCode || !encodedResourceCode) return null;

  let workspaceCode: string;
  let resourceCode: string;
  try {
    workspaceCode = decodeURIComponent(encodedWorkspaceCode);
    resourceCode = decodeURIComponent(encodedResourceCode);
  } catch {
    return null;
  }

  return { resourceCode, workspaceCode };
}

export function buildAvatarResourceReference(reference: AvatarResourceReference) {
  return `${AVATAR_RESOURCE_PREFIX}${encodeURIComponent(reference.workspaceCode)}/${encodeURIComponent(reference.resourceCode)}`;
}

export function isAvatarResourceReference(value: string | null | undefined) {
  return parseAvatarResourceReference(value) !== null;
}
