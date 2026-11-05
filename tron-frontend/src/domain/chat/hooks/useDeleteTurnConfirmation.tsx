import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "@/components/confirm-dialog";

export function useDeleteTurnConfirmation(sessionId: string | null) {
  const { t } = useTranslation("pages");
  const [open, setOpen] = useState(false);
  const pending = useRef<((value: boolean) => void) | null>(null);
  const settle = useCallback((value: boolean) => {
    pending.current?.(value);
    pending.current = null;
    setOpen(false);
  }, []);
  useEffect(() => {
    settle(false);
    return () => { pending.current?.(false); pending.current = null; };
  }, [sessionId, settle]);
  const confirm = useCallback(() => new Promise<boolean>(resolve => {
    pending.current?.(false);
    pending.current = resolve;
    setOpen(true);
  }), []);
  const dialog = <ConfirmDialog open={open} title={t("chat.management.deleteTurn.title")}
    description={t("chat.management.deleteTurn.description")} cancelText={t("chat.management.deleteTurn.cancel")}
    confirmText={t("chat.management.deleteTurn.action")} onCancel={() => settle(false)} onConfirm={() => settle(true)} />;
  return { confirm, dialog };
}
