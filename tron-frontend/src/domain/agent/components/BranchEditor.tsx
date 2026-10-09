import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import type { BranchCondition, LoopDesign } from "../model/LoopModel";
import { loopNodeName } from "../model/builtinLoops";

type ValueType = "boolean" | "string" | "number" | "null" | "json";
function valueType(value: unknown): ValueType {
  if (value === null) return "null";
  if (["boolean", "string", "number"].includes(typeof value))
    return typeof value as ValueType;
  return "json";
}

export function BranchEditor({
  condition,
  design,
  onChange,
  onInvalid,
}: {
  condition: BranchCondition;
  design: LoopDesign;
  onChange: (condition: BranchCondition) => void;
  onInvalid: (invalid: boolean) => void;
}) {
  const { t } = useTranslation("pages");
  const [type, setType] = useState<ValueType>(() =>
    valueType(condition.equals),
  );
  const [text, setText] = useState(() =>
    typeof condition.equals === "string"
      ? condition.equals
      : JSON.stringify(condition.equals),
  );
  const nodeName = (id: string) => {
    const node = design.nodes.find((node) => node.id === id);
    return node ? loopNodeName(design, node, t) : "—";
  };
  const route = (key: "sourceNode" | "matchTarget" | "elseTarget") => (
    <Field label={t(`agentLoops.${key}`)}>
      <span className="break-words text-sm">{nodeName(condition[key])}</span>
    </Field>
  );
  const pointer = condition.pointer;
  const simple = pointer.startsWith("/") && !pointer.slice(1).includes("/");
  const [field, setField] = useState(() =>
    simple ? pointer.slice(1).replace(/~1/g, "/").replace(/~0/g, "~") : pointer,
  );
  const equals =
    type === "boolean"
      ? t(condition.equals ? "agentLoops.yes" : "agentLoops.no")
      : JSON.stringify(condition.equals);
  return (
    <div className="space-y-4">
      {route("sourceNode")}
      <Field label={t("agentLoops.pointer")}>
        <TextInput
          aria-label={t("agentLoops.pointer")}
          value={field}
          placeholder="approved"
          onChange={(event) => {
            const value = event.target.value;
            setField(value);
            onChange({
              ...condition,
              pointer:
                !value || value.startsWith("/")
                  ? value
                  : `/${value.replace(/~/g, "~0").replace(/\//g, "~1")}`,
            });
          }}
        />
        <p className="mt-2 text-xs leading-relaxed text-[var(--color-on-surface-variant)]">
          {t("agentLoops.pointerHelp")}
        </p>
        <details className="mt-2 text-xs text-[var(--color-on-surface-variant)]">
          <summary className="cursor-pointer">
            {t("agentLoops.nestedField")}
          </summary>
          <p className="mt-2 leading-relaxed">
            {t("agentLoops.nestedFieldHelp")}
          </p>
        </details>
      </Field>
      <Field label={t("agentLoops.valueType")}>
        <SelectInput
          aria-label={t("agentLoops.valueType")}
          value={type}
          onValueChange={(value) => {
            const next = value as ValueType;
            const defaults = {
              boolean: true,
              string: "",
              number: 0,
              null: null,
              json: {},
            };
            setType(next);
            setText(next === "string" ? "" : JSON.stringify(defaults[next]));
            onInvalid(false);
            onChange({ ...condition, equals: defaults[next] });
          }}
        >
          {(["boolean", "string", "number", "null", "json"] as const).map(
            (value) => (
              <SelectItem key={value} value={value}>
                {t(`agentLoops.valueTypes.${value}`)}
              </SelectItem>
            ),
          )}
        </SelectInput>
      </Field>
      {type !== "null" ? (
        <Field label={t("agentLoops.equals")}>
          {type === "boolean" ? (
            <SelectInput
              aria-label={t("agentLoops.equals")}
              value={String(condition.equals)}
              onValueChange={(value) =>
                onChange({ ...condition, equals: value === "true" })
              }
            >
              <SelectItem value="true">{t("agentLoops.yes")}</SelectItem>
              <SelectItem value="false">{t("agentLoops.no")}</SelectItem>
            </SelectInput>
          ) : (
            <TextInput
              aria-label={t("agentLoops.equals")}
              value={text}
              type={type === "number" ? "number" : "text"}
              step="any"
              onChange={(event) => {
                const value = event.target.value;
                setText(value);
                try {
                  const parsed: unknown =
                    type === "string" ? value : JSON.parse(value);
                  if (
                    type === "number" &&
                    (typeof parsed !== "number" || !Number.isFinite(parsed))
                  )
                    throw new Error("number");
                  onInvalid(false);
                  onChange({ ...condition, equals: parsed });
                } catch {
                  onInvalid(true);
                }
              }}
            />
          )}
        </Field>
      ) : null}
      <p
        className="rounded-lg bg-[var(--color-surface-container-low)] p-3 text-xs leading-relaxed"
        aria-live="polite"
      >
        {t("agentLoops.branchSummary", {
          source: nodeName(condition.sourceNode),
          field: field || t("agentLoops.wholeOutput"),
          value: equals,
          target: nodeName(condition.matchTarget),
          otherwise: nodeName(condition.elseTarget),
        })}
      </p>
      <div className="grid gap-3">
        {route("matchTarget")}
        {route("elseTarget")}
      </div>
    </div>
  );
}
