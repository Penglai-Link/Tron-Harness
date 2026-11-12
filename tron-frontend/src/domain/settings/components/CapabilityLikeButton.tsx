import { useRef } from "react";
import {
  useIsMutating,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { LoaderCircle, ThumbsUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import {
  setCenterLike,
  type CenterDetail,
  type CenterPack,
  type CenterPage,
} from "../api/capabilityCenterApi";

export function CapabilityLikeButton({
  pack,
  disabled,
}: {
  pack: CenterPack;
  disabled: boolean;
}) {
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const toast = useToast();
  const client = useQueryClient();
  const lock = useRef(false);
  const mutationKey = ["capability-center", "like", pack.packId];
  const pending = useIsMutating({ mutationKey }) > 0;
  const mutation = useMutation({
    mutationKey,
    mutationFn: (liked: boolean) => setCenterLike(pack.packId, liked),
    retry: false,
    onSuccess: async (state) => {
      // Stop older reads from overwriting the confirmed state, then reconcile server ordering.
      await client.cancelQueries({
        queryKey: ["capability-center"],
        predicate: (query) =>
          ["page", "detail"].includes(String(query.queryKey[1])),
      });
      const update = (item: CenterPack): CenterPack =>
        item.packId === state.packId
          ? { ...item, likeCount: state.likeCount, liked: state.liked }
          : item;
      client.setQueriesData<CenterPage>(
        { queryKey: ["capability-center", "page"] },
        (data) => data && { ...data, records: data.records.map(update) }
      );
      client.setQueriesData<CenterDetail>(
        { queryKey: ["capability-center", "detail"] },
        (data) => data && { ...data, pack: update(data.pack) }
      );
    },
    onError: (error) =>
      toast.error(error.message, { title: t("capabilityCenter.likeFailed") }),
    onSettled: () =>
      client.invalidateQueries({
        queryKey: ["capability-center"],
        predicate: (query) =>
          ["page", "detail"].includes(String(query.queryKey[1])),
      }),
  });
  const label = t(
    pack.liked ? "capabilityCenter.unlikePack" : "capabilityCenter.likePack",
    { name: pack.name, value: formatNumber(pack.likeCount ?? 0) }
  );
  return (
    <Button
      className="pointer-events-auto gap-1.5 tabular-nums"
      variant={pack.liked ? "default" : "outline"}
      size="sm"
      disabled={disabled || pending || pack.liked == null}
      aria-label={label}
      title={label}
      aria-pressed={pack.liked ?? false}
      aria-busy={pending}
      onClick={async () => {
        if (lock.current || pending || pack.liked == null) return;
        lock.current = true;
        try {
          await mutation.mutateAsync(!pack.liked);
        } catch {
          // The mutation reports the error; retain the last confirmed state.
        } finally {
          lock.current = false;
        }
      }}
    >
      {pending ? (
        <LoaderCircle
          size={14}
          aria-hidden="true"
          className="animate-spin motion-reduce:animate-none"
        />
      ) : (
        <ThumbsUp
          size={14}
          aria-hidden="true"
          className={pack.liked ? "fill-current" : undefined}
        />
      )}
      {formatNumber(pack.likeCount ?? 0)}
    </Button>
  );
}
