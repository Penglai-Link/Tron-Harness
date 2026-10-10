import { useTranslation } from "react-i18next";
import { Field, SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { useLoopDesignList } from "../hooks/useLoopDesignList";
import { loopName } from "../model/builtinLoops";
export function AgentLoopField({
  value,
  onChange,
  disabled,
}: {
  value?: string | null;
  onChange: (code: string) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("pages");
  const { designs, loading, error } = useLoopDesignList();
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
