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

  const saveProfile = useCallback(async (payload: UpdateAccountProfilePayload) => {
    setSavingProfile(true);
    try {
      const updated = await updateAccountProfile(payload);
      setProfile(updated);
      updateSessionUser({
        displayName: updated.displayName,
        email: updated.email,
        passwordResetRequired: updated.passwordResetRequired,
      });
      return updated;
    } finally {
      setSavingProfile(false);
    }
  }, [updateSessionUser]);

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
