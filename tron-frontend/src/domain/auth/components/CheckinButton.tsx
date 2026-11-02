import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useAuthStore } from "@/store/authStore";
import { fetchBootstrapProfileStatus } from "@/domain/profile/api/profileApi";
import { checkinApi } from "../api/checkinApi";

export function CheckinButton() {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const userCode = useAuthStore((state) => state.session?.user.userCode);
  const [checked, setChecked] = useState<boolean | null>(null);
  const [statusError, setStatusError] = useState(false);
  const [revision, setRevision] = useState(0);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  useEffect(() => {
    let disposed = false;
    const refresh = () => { void checkinApi.status().then((result) => {
      if (!disposed && !busy.current) { setChecked(result.todayCheckedIn); setStatusError(false); }
    }).catch(() => { if (!disposed && !busy.current) setStatusError(true); }); };
    setChecked(null);
    setStatusError(false);
    refresh();
    window.addEventListener("focus", refresh);
    const timer = window.setInterval(refresh, 60_000);
    return () => { disposed = true; window.clearInterval(timer); window.removeEventListener("focus", refresh); };
  }, [userCode, revision]);

  async function checkin() {
    if (busy.current || checked !== false) return;
    busy.current = true;
    setPending(true);
    try {
      const result = await checkinApi.checkin();
      setChecked(true);
      toast.success(result.points === null ? t("userMenu.checkinDone") : t("userMenu.checkinSuccess", { points: result.points }));
      try {
        const [profile, status] = await Promise.all([fetchBootstrapProfileStatus(), checkinApi.status()]);
        if (useAuthStore.getState().session?.user.userCode === userCode) {
          setChecked(status.todayCheckedIn);
          if (profile.activeProfile?.personAiUserId === userCode && profile.account) {
            useAuthStore.getState().updateSessionUser(profile.account);
          }
        }
      } catch { toast.error(t("userMenu.checkinBalanceFailed")); }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("userMenu.checkinFailed"));
      try { setChecked((await checkinApi.status()).todayCheckedIn); } catch { /* Keep retry available. */ }
    } finally { busy.current = false; setPending(false); }
  }
  return <Button variant="outline" size="sm" disabled={pending || checked === true || (checked === null && !statusError)} onClick={() => { if (checked === null) setRevision((value) => value + 1); else void checkin(); }}>
    {pending ? t("userMenu.checkingIn") : checked === null ? t(statusError ? "userMenu.checkinStatusRetry" : "userMenu.checkinStatusLoading") : checked ? t("userMenu.checkedIn") : t("userMenu.checkin")}
  </Button>;
}
