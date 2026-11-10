import { useTranslation } from "react-i18next";
import { Field, TextInput, TextArea, SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import type { MemoryTarget } from "../model";
export function MemoryFormFields({
  title,
  text,
  target,
  targets,
  locked = false,
  disabled = false,
  onTitle,
  onText,
  onTarget,
}: {
  title: string;
  text: string;
  target: string;
  targets: MemoryTarget[];
  locked?: boolean;
  disabled?: boolean;
  onTitle: (v: string) => void;
  onText: (v: string) => void;
  onTarget: (v: string) => void;
}) {
  const { t } = useTranslation("pages");
  return (
    <div className="max-w-4xl space-y-5">
      <Field label={t("memory.simple.ownership")}>
        <SelectInput
          aria-label={t("memory.simple.ownership")}
          disabled={disabled || locked}
          value={target}
          onValueChange={(v) => onTarget(String(v))}
        >
          {targets.map((v) => (
            <SelectItem
              key={`${v.scope}:${v.scopeCode}`}
              value={`${v.scope}:${v.scopeCode}`}
            >
              {t(`memory.simple.scope.${v.scope}`)} · {v.label}
            </SelectItem>
          ))}
        </SelectInput>
      </Field>
      <Field label={t("memory.simple.title")}>
        <TextInput
          aria-label={t("memory.simple.title")}
          disabled={disabled}
          value={title}
          maxLength={1000}
          onChange={(e) => onTitle(e.target.value)}
        />
      </Field>
      <Field required label={t("memory.simple.content")}>
        <TextArea
          aria-label={t("memory.simple.content")}
          disabled={disabled}
          className="min-h-80"
          value={text}
          onChange={(e) => onText(e.target.value)}
        />
      </Field>
    </div>
  );
}
