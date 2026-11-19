import { useCallback, useEffect, useMemo, useRef } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useTabVisibility } from "@/components/editable-tabs";
import { useToast } from "@/components/ui/toast";
import { notifyCapabilityResourcesChanged } from "@/domain/skill/api";
import type { CenterPack } from "../api/capabilityCenterApi";
import {
  enqueueCapabilityOperation,
  getCapabilityQueue,
  type CapabilityQueueOperation,
} from "../api/capabilityQueueApi";

const QUEUE_KEY = ["capability-queue"] as const;
const CENTER_KEY = ["capability-center"] as const;

interface EnqueueInput {
  packId: string;
  packName: string;
  operation: CapabilityQueueOperation;
  force: boolean;
}

export function useCapabilityQueue() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const client = useQueryClient();
  const active = useTabVisibility();
  const query = useQuery({
    queryKey: QUEUE_KEY,
    queryFn: ({ signal }) => getCapabilityQueue(signal),
    retry: false,
    refetchInterval: (current) =>
      active && (current.state.data?.items.length ?? 0) > 0 ? 3000 : false,
  });
  // Background execution finishes while the user may be elsewhere. Watching the
  // log lets the open page refresh pack state and navigation exactly once per
  // completed operation instead of polling the center list.
  const seenLogIds = useRef<Set<string> | null>(null);
  useEffect(() => {
    const log = query.data?.log;
    if (!log) return;
    const ids = new Set(log.map((entry) => entry.id));
    const previous = seenLogIds.current;
    seenLogIds.current = ids;
    if (!previous || !log.some((entry) => !previous.has(entry.id))) return;
    notifyCapabilityResourcesChanged();
    window.dispatchEvent(new Event("tron:navigation-updated"));
    void client.invalidateQueries({ queryKey: CENTER_KEY });
  }, [query.data, client]);
  const mutation = useMutation({
    mutationFn: (input: EnqueueInput) => enqueueCapabilityOperation(input),
    onSuccess: (result, input) => {
      if (result.status === "succeeded") {
        const notice =
          input.operation === "uninstall"
            ? "uninstalledNotice"
            : input.operation === "enable"
            ? "enabledNotice"
            : "disabledNotice";
        toast.success(t(`capabilityCenter.${notice}`));
        notifyCapabilityResourcesChanged();
        window.dispatchEvent(new Event("tron:navigation-updated"));
      } else if (result.status === "failed") {
        toast.error(result.error ?? t("capabilityCenter.queue.failed"), {
          title: t("capabilityCenter.operationFailed"),
        });
      } else {
        toast.info(t("capabilityCenter.queue.queuedNotice"));
      }
    },
    onError: (error) =>
      toast.error(error.message, {
        title: t("capabilityCenter.queue.enqueueFailed"),
      }),
    onSettled: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: QUEUE_KEY }),
        client.invalidateQueries({ queryKey: CENTER_KEY }),
      ]);
    },
  });
  const pendingPackIds = useMemo(
    () => new Set((query.data?.items ?? []).map((item) => item.packId)),
    [query.data?.items]
  );
  const enqueue = useCallback(
    (
      pack: Pick<CenterPack, "packId" | "name">,
      operation: CapabilityQueueOperation,
      force = false
    ) =>
      mutation.mutateAsync({
        packId: pack.packId,
        packName: pack.name,
        operation,
        force,
      }),
    [mutation]
  );
  const toggle = useCallback(
    (pack: Pick<CenterPack, "packId" | "name" | "enabled">) => {
      if (mutation.isPending) return;
      void enqueue(pack, pack.enabled ? "disable" : "enable").catch(() => {});
    },
    [enqueue, mutation.isPending]
  );
  const uninstall = useCallback(
    (pack: Pick<CenterPack, "packId" | "name">, force = false) =>
      enqueue(pack, "uninstall", force),
    [enqueue]
  );
  return {
    query,
    toggle,
    uninstall,
    pending: mutation.isPending,
    pendingPackIds,
  };
}
