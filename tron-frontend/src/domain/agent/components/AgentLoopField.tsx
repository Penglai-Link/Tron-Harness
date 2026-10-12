import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { useLoopDesignList } from "../hooks/useLoopDesignList";
import { loopName } from "../model/builtinLoops";
import type { LoopKind } from "../model/LoopModel";
export function AgentLoopField({
  value,
  onChange,
  disabled,
  defaultKind,
}: {
  value?: string | null;
  onChange: (code: string) => void;
  disabled?: boolean;
  defaultKind?: LoopKind;
}) {
  const { t } = useTranslation("pages");
  const { designs, loading, error } = useLoopDesignList();
  useEffect(() => {
    if (!value && !loading && !error && defaultKind) {
      const defaultDesign = designs.find((design) => design.kind === defaultKind);
      if (defaultDesign) onChange(defaultDesign.loopCode);
    }
  }, [defaultKind, designs, error, loading, onChange, value]);
  return (
    <Field label={t("agentLoops.selection", { defaultValue: "Agent Loop" })}>
      <SelectInput
        aria-label={t("agentLoops.selection", { defaultValue: "Agent Loop" })}
        disabled={disabled || loading || Boolean(error)}
        value={value || ""}
        onValueChange={(code) => {
          if (code) onChange(code);
        }}
      >
        <SelectItem value="" disabled>
          {t("agentLoops.selectPlaceholder", { defaultValue: "Select an agent loop" })}
        </SelectItem>
        {designs.map((d) => (
          <SelectItem key={d.loopCode} value={d.loopCode}>
            {loopName(d)} · {t(`agentLoops.kinds.${d.kind}`)}
          </SelectItem>
        ))}
      </SelectInput>
      {error ? (
        <p role="alert" className="text-sm text-[var(--color-error)]">
          {error}
        </p>
      ) : null}
    </Field>
  );
}
