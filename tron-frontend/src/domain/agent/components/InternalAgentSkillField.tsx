import { reconcileResourceDraft } from "../model/ResourceAssignmentDraft";
import { useState, useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Field } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { SearchField } from "@/components/search-field";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  fetchAgentSkills,
  fetchRegistrySkills,
  CAPABILITY_RESOURCES_CHANGED,
} from "@/domain/skill/api";
import type { RegistrySkillRecord } from "@/domain/skill/model";

export function DisabledResourceReason({ reason }: { reason?: string | null }) {
  return reason ? (
    <Tooltip>
      <TooltipTrigger
        type="button"
        aria-label={reason}
        className="text-xs text-[var(--color-on-surface-variant)]"
      >
        ⓘ
      </TooltipTrigger>
      <TooltipContent>{reason}</TooltipContent>
    </Tooltip>
  ) : null;
}
export function InternalAgentSkillField({
  agentCode,
  disabled,
  selectedNames,
  onSelectionChange,
}: {
  agentCode: string;
  disabled: boolean;
  selectedNames?: string[];
  onSelectionChange: (names: string[]) => void;
}) {
  const { t } = useTranslation("pages");
  const [open, setOpen] = useState(false);
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["agent-skills", agentCode],
    queryFn: () => agentCode ? fetchAgentSkills(agentCode) : fetchRegistrySkills().then((items) => items.map((item) => ({ ...item, assigned: item.available && !!item.providerPackId }))),
  });
  useEffect(() => {
    const changed = () => {
      void client.invalidateQueries({ queryKey: ["agent-skills"] });
    };
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
    return () =>
      window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
  }, [client]);
  const previousItems = useRef<RegistrySkillRecord[] | undefined>(undefined);
  useEffect(() => {
    const previous = previousItems.current;
    previousItems.current = query.data;
    if (!query.data || !selectedNames) return;
    const next = reconcileResourceDraft(
      selectedNames,
      (previous ?? query.data)
        .filter((item) => item.assigned)
        .map((item) => item.skillName),
      query.data.filter((item) => item.assigned).map((item) => item.skillName),
      query.data.filter((item) => item.available).map((item) => item.skillName)
    );
    if (
      next.length !== selectedNames.length ||
      next.some((name) => !selectedNames.includes(name))
    ) {
      onSelectionChange(next);
    }
  }, [query.data, selectedNames, onSelectionChange]);
  const selected =
    query.data?.filter(
      (s) =>
        s.available &&
        (selectedNames ? selectedNames.includes(s.skillName) : s.assigned)
    ) ?? [];
  return (
    <>
      <Field
        className="w-full lg:col-span-2"
        label={t("agentSkills.label")}
        actions={
          <Button
            size="sm"
            variant="outline"
            disabled={disabled || query.isPending || !!query.error}
            onClick={() => setOpen(true)}
          >
            {t("agentSkills.choose", { count: selected.length })}
          </Button>
        }
      >
        {query.error ? (
          <p role="alert">
            {query.error.message}
            <Button
              size="sm"
              variant="outline"
              onClick={() => void query.refetch()}
            >
              {t("agentSkills.retry")}
            </Button>
          </p>
        ) : (
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {selected.map((s) => s.skillName).join(" · ") ||
              t("agentSkills.none")}
          </p>
        )}
      </Field>
      {open && query.data ? (
        <SkillSelectionDialog
          key={agentCode}
          items={query.data}
          selected={selected.map((s) => s.skillName)}
          onClose={() => setOpen(false)}
          onConfirm={(names) => {
            onSelectionChange(names);
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}
export function SkillSelectionDialog({
  items,
  selected,
  onClose,
  onConfirm,
}: {
  items: RegistrySkillRecord[];
  selected: string[];
  onClose: () => void;
  onConfirm: (names: string[]) => void;
}) {
  const { t } = useTranslation("pages");
  const [names, setNames] = useState(selected);
  const [search, setSearch] = useState("");
  const previousItems = useRef(items);
  useEffect(() => {
    const previous = previousItems.current;
    previousItems.current = items;
    setNames((current) => {
      const next = reconcileResourceDraft(
        current,
        previous.filter((item) => item.assigned).map((item) => item.skillName),
        items.filter((item) => item.assigned).map((item) => item.skillName),
        items.filter((item) => item.available).map((item) => item.skillName)
      );
      return next.length === current.length &&
        next.every((name) => current.includes(name))
        ? current
        : next;
    });
  }, [items]);
  const available = items.filter((s) => s.available);
  const effective = names.filter((n) =>
    available.some((s) => s.skillName === n)
  );
  const visible = items.filter((s) =>
    `${s.skillName} ${s.description ?? ""} ${s.sourceRef}`
      .toLowerCase()
      .includes(search.toLowerCase())
  );
  return (
    <Dialog
      disablePointerDismissal
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className="flex max-h-[85dvh] flex-col sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("agentSkills.title")}</DialogTitle>
          <DialogDescription>{t("agentSkills.draftHint")}</DialogDescription>
        </DialogHeader>
        <SearchField
          ariaLabel={t("agentSkills.search")}
          placeholder={t("agentSkills.search")}
          value={search}
          onChange={setSearch}
        />
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              setNames([
                ...new Set([
                  ...effective,
                  ...visible.filter((s) => s.available).map((s) => s.skillName),
                ]),
              ])
            }
          >
            {t("agentSkills.selectAll")}
          </Button>
          <Button size="sm" variant="outline" onClick={() => setNames([])}>
            {t("agentSkills.clear")}
          </Button>
        </div>
        <div className="min-h-0 space-y-2 overflow-y-auto">
          {visible.map((s) => (
            <div
              key={s.skillName}
              className={`rounded-xl border border-[var(--color-outline-variant)] p-3 ${
                s.available ? "" : "opacity-60"
              }`}
            >
              <label className="flex items-start gap-3">
                <input
                  type="checkbox"
                  className="shrink-0"
                  aria-label={s.skillName}
                  disabled={!s.available}
                  checked={s.available && effective.includes(s.skillName)}
                  onChange={(e) =>
                    setNames(
                      e.currentTarget.checked
                        ? [...effective, s.skillName]
                        : effective.filter((n) => n !== s.skillName)
                    )
                  }
                />
                <span className="min-w-0 break-words">
                  <span className="block text-sm font-medium">
                    {s.skillName}
                  </span>
                  <span className="block text-xs text-[var(--color-on-surface-variant)]">
                    {s.description || s.sourceRef}
                  </span>
                </span>
              </label>
              <DisabledResourceReason reason={s.disabledReason} />
            </div>
          ))}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("agentSkills.cancel")}
          </Button>
          <Button onClick={() => onConfirm(effective)}>
            {t("agentSkills.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
