import { useMutation } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { updateProfileTimeZone } from "@/domain/profile/api";
import { useTimeZoneStore } from "../lib/userTimeZone";
export function useUpdateUserTimeZone() {
  const toast = useToast();
  const { t } = useTranslation("shell");
  const timeZone = useTimeZoneStore((s) => s.timeZone);
  const mutation = useMutation({
    mutationFn: updateProfileTimeZone,
    onSuccess: () => window.location.reload(),
    onError: () => toast.error(t("userMenu.timeZoneSaveFailed")),
  });
  return {
    timeZone,
    saving: mutation.isPending,
    setTimeZone: (zone: string) => {
      if (zone !== timeZone) mutation.mutate(zone);
    },
  };
}
