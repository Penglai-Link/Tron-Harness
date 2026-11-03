import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useAuthStore } from "@/store/authStore";
import { checkinApi } from "../api/checkinApi";

export function CheckinButton() {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const userCode = useAuthStore((state) => state.session?.user.userCode);
  const checked = useAuthStore((state) => state.session?.user.todayCheckedIn ?? null);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);

  async function checkin() {
    if (busy.current || checked !== false || !userCode) return;
    busy.current = true;
    setPending(true);
    try {
      const result = await checkinApi.checkin();
      if (useAuthStore.getState().session?.user.userCode !== userCode) return;
      useAuthStore.getState().updateSessionUser({ todayCheckedIn: true });
      toast.success(result.points === null ? t("userMenu.checkinDone") : t("userMenu.checkinSuccess", { points: result.points }));
      await useAuthStore.getState().refreshAccountSnapshot();
    } catch (error) {
      if (useAuthStore.getState().session?.user.userCode !== userCode) return;
      toast.error(error instanceof Error ? error.message : t("userMenu.checkinFailed"));
      await useAuthStore.getState().refreshAccountSnapshot();
    } finally { busy.current = false; setPending(false); }
  }

  return <Button variant="outline" size="sm" disabled={pending || checked !== false} onClick={() => void checkin()}>
    {pending ? t("userMenu.checkingIn") : checked === null ? "—" : checked ? t("userMenu.checkedIn") : t("userMenu.checkin")}
  </Button>;
}
