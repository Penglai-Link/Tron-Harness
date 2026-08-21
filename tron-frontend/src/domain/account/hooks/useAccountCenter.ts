import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  changeAccountPassword,
  fetchAccountProfile,
  updateAccountProfile,
} from "@/domain/account/api";
import {
  createResource,
  uploadResourceBinary,
} from "@/domain/resource/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  buildAvatarResourceReference,
  parseAvatarResourceReference,
} from "@/shared/lib/avatarResource";
import type {
  AccountProfile,
  ChangeAccountPasswordPayload,
  UpdateAccountProfilePayload,
} from "@/domain/account/model";
import { useAuthStore } from "@/store/authStore";

export function useAccountCenter() {
  const updateSessionUser = useAuthStore((state) => state.updateSessionUser);
  const [profile, setProfile] = useState<AccountProfile | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<unknown>(null);
  const [savingProfile, setSavingProfile] = useState(false);
  const [changingPassword, setChangingPassword] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setLoadError(null);

    void fetchAccountProfile(controller.signal)
      .then((nextProfile) => {
        setProfile(nextProfile);
      })
      .catch((error: unknown) => {
        if (!controller.signal.aborted) {
          setLoadError(error);
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, [refreshKey]);

  const refresh = useCallback(() => {
    setRefreshKey((value) => value + 1);
  }, []);

  const saveProfile = useCallback(async (
    payload: UpdateAccountProfilePayload,
    avatarFile?: File,
    avatarWorkspaceCode?: string,
  ) => {
    setSavingProfile(true);
    try {
      let avatarUrl = payload.avatarUrl;
      if (avatarFile) {
        if (getServiceDataSource("resource") !== "api") {
          avatarUrl = URL.createObjectURL(avatarFile);
        } else {
          if (!avatarWorkspaceCode?.trim()) {
            throw new Error("Select a workspace before uploading an avatar.");
          }

          const previousReference = parseAvatarResourceReference(profile?.avatarUrl);
          const nextReference = previousReference?.workspaceCode === avatarWorkspaceCode
            ? previousReference
            : {
                resourceCode: `avatar-${profile?.userCode ?? "user"}-${Date.now()}`
                  .replace(/[^A-Za-z0-9_.-]/g, "-")
                  .slice(0, 64),
                workspaceCode: avatarWorkspaceCode,
              };

          if (previousReference && previousReference.workspaceCode === avatarWorkspaceCode) {
            await uploadResourceBinary({
              file: avatarFile,
              resourceCode: nextReference.resourceCode,
              workspaceCode: nextReference.workspaceCode,
            });
          } else {
            await createResource({
              allowInheritance: false,
              description: "Managed account avatar",
              file: avatarFile,
              kind: "IMAGE",
              name: `Avatar ${profile?.displayName ?? "User"}`,
              resourceCode: nextReference.resourceCode,
              workspaceCode: nextReference.workspaceCode,
            });
          }

          avatarUrl = buildAvatarResourceReference(nextReference);
        }
      }

      const updated = await updateAccountProfile({ ...payload, avatarUrl });
      setProfile(updated);
      updateSessionUser({
        avatarUrl: updated.avatarUrl,
        displayName: updated.displayName,
        email: updated.email,
        passwordResetRequired: updated.passwordResetRequired,
      });
      return updated;
    } finally {
      setSavingProfile(false);
    }
  }, [profile, updateSessionUser]);

  const savePassword = useCallback(async (payload: ChangeAccountPasswordPayload) => {
    setChangingPassword(true);
    try {
      const result = await changeAccountPassword(payload);
      setProfile((current) => current ? {
        ...current,
        passwordResetRequired: result.passwordResetRequired,
      } : current);
      updateSessionUser({ passwordResetRequired: result.passwordResetRequired });
      return result;
    } finally {
      setChangingPassword(false);
    }
  }, [updateSessionUser]);

  return {
    changePassword: savePassword,
    changingPassword,
    loadError,
    loading,
    profile,
    refresh,
    saveProfile,
    savingProfile,
  };
}
