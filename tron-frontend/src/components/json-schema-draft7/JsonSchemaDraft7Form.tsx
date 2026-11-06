import { Trash2, Braces, KeyRound } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { cn } from "@/shared/lib/utils";
import {
  Field,
  SelectInput,
  TextArea,
  TextInput,
  ToggleSwitch,
} from "@/components/forms";

import {
  asSchemaObject,
  cloneJsonSchema,
} from "./schema";
import type {
  JsonSchemaDraft7,
  JsonSchemaDraft7RootObject,
  JsonSchemaDraft7Type,
  JsonSchemaDraft7Value,
} from "./types";

export interface JsonSchemaDraft7VariableOption {
  label: string;
  value: string;
}

export interface JsonSchemaDraft7FormProps {
  className?: string;
  errors?: string[];
  onChange: (value: Record<string, unknown>) => void;
  readOnly?: boolean;
  schema: JsonSchemaDraft7RootObject;
  value: Record<string, unknown>;
  variableOptions?: JsonSchemaDraft7VariableOption[];
}

interface ValueEditorProps {
  depth: number;
  errors: string[];
  label: string;
  onChange: (value: unknown) => void;
  path: string;
  readOnly: boolean;
  required: boolean;
  schemaValue: JsonSchemaDraft7Value;
  value: unknown;
  variableOptions: JsonSchemaDraft7VariableOption[];
}

const defaultVariableOptions: JsonSchemaDraft7VariableOption[] = [
  { label: "DATABASE_USERNAME", value: "DATABASE_USERNAME" },
  { label: "DATABASE_PASSWORD", value: "DATABASE_PASSWORD" },
  { label: "MYSQL_CA_CERTIFICATE", value: "MYSQL_CA_CERTIFICATE" },
  { label: "MYSQL_CLIENT_CERTIFICATE", value: "MYSQL_CLIENT_CERTIFICATE" },
  { label: "MYSQL_CLIENT_KEY", value: "MYSQL_CLIENT_KEY" },
];

function declaredTypes(schema: JsonSchemaDraft7): JsonSchemaDraft7Type[] {
  if (Array.isArray(schema.type)) return schema.type;
  if (schema.type) return [schema.type];
  if (schema.properties) return ["object"];
  if (schema.items) return ["array"];
  return ["string"];
}

function inferredType(schema: JsonSchemaDraft7, value: unknown): JsonSchemaDraft7Type {
  const types = declaredTypes(schema);
  if (types.length === 1) return types[0];
  if (value === null && types.includes("null")) return "null";
  if (Array.isArray(value) && types.includes("array")) return "array";
  if (typeof value === "boolean" && types.includes("boolean")) return "boolean";
  if (typeof value === "number") {
    if (Number.isInteger(value) && types.includes("integer")) return "integer";
    if (types.includes("number")) return "number";
  }
  if (value && typeof value === "object" && types.includes("object")) return "object";
  if (typeof value === "string" && types.includes("string")) return "string";
  return types.find((type) => type !== "null") ?? types[0];
}

function neutralValue(schemaValue: JsonSchemaDraft7Value): unknown {
  if (schemaValue === false) return null;
  if (schemaValue === true) return {};
  const schema = schemaValue;
  if (schema.default !== undefined) return cloneJsonSchema(schema.default);
  if (schema.const !== undefined) return cloneJsonSchema(schema.const);
  if (schema.enum?.length) return cloneJsonSchema(schema.enum[0]);
  if (schema.oneOf?.length) return neutralValue(schema.oneOf[0]);
  if (schema.anyOf?.length) return neutralValue(schema.anyOf[0]);
  if (schema.allOf?.length && !schema.type && !schema.properties) return neutralValue(schema.allOf[0]);

  const type = declaredTypes(schema).find((item) => item !== "null") ?? "null";
  if (type === "object") {
    return Object.fromEntries(Object.entries(schema.properties ?? {}).map(([name, childSchema]) => (
      [name, neutralValue(childSchema)]
    )));
  }
  if (type === "array") {
    if (!schema.items || Array.isArray(schema.items)) return [];
    return Array.from({ length: schema.minItems ?? 0 }, () => neutralValue(schema.items as JsonSchemaDraft7Value));
  }
  if (type === "boolean") return false;
  if (type === "integer" || type === "number") return schema.minimum ?? 0;
  if (type === "null") return null;
  return "";
}

export function createJsonSchemaDraft7FormValue(schema: JsonSchemaDraft7RootObject) {
  return neutralValue(schema) as Record<string, unknown>;
}

function inputError(errors: string[], path: string) {
  return errors.find((error) => error.startsWith(`${path}:`))?.replace(`${path}: `, "");
}

function propertyPath(path: string, name: string) {
  return path === "$" ? `$.${name}` : `${path}.${name}`;
}

function itemPath(path: string, index: number) {
  return `${path}[${index}]`;
}

function fieldLabel(label: string, required: boolean) {
  return (
    <span>
      {label}
      {required ? <span aria-hidden="true" className="ml-1 text-[var(--color-error)]">*</span> : null}
    </span>
  );
}

function NumericValueInput({ value, onChange, integer, label, readOnly, placeholder }: {
  value: unknown; onChange: (value: unknown) => void; integer: boolean; label: string; readOnly: boolean; placeholder?: string;
}) {
  const [draft, setDraft] = useState(typeof value === "number" ? String(value) : "");
  useEffect(() => {
    if (typeof value === "number") {
      setDraft((current) => current && Number(current) === value ? current : String(value));
    } else if (value == null) {
      setDraft((current) => current === "-" || current.endsWith(".") ? current : "");
    }
  }, [value]);
  return <TextInput aria-label={label} disabled={readOnly} inputMode={integer ? "numeric" : "decimal"}
    placeholder={placeholder} type="text" value={draft} onChange={(event) => {
      const next = event.target.value;
      if (!(integer ? /^-?\d*$/ : /^-?\d*(?:\.\d*)?$/).test(next)) return;
      setDraft(next);
      const complete = /\d$/.test(next) && Number.isFinite(Number(next));
      onChange(complete ? Number(next) : null);
    }} />;
}

function PrimitiveValueEditor({
  errors,
  label,
  onChange,
  path,
  readOnly,
  required,
  schema,
  type,
  value,
  variableOptions,
}: Omit<ValueEditorProps, "depth" | "schemaValue"> & {
  schema: JsonSchemaDraft7;
  type: JsonSchemaDraft7Type;
}) {
  const { t } = useTranslation("common");
  const error = inputError(errors, path);
  const hint = schema.description;

  if (schema.enum?.length) {
    const options = schema.enum.map((option, index) => ({ key: String(index), option }));
    const selected = options.find((option) => Object.is(option.option, value))?.key ?? "";
    return (
      <Field error={error} hint={hint} label={fieldLabel(label, required)}>
        <SelectInput
          aria-label={label}
          className="w-full"
          disabled={readOnly}
          placeholder={t("jsonSchemaForm.selectValue", { defaultValue: "Select a value" })}
          value={selected}
          onValueChange={(key) => {
            const option = options.find((item) => item.key === key);
            if (option) onChange(cloneJsonSchema(option.option));
          }}
        >
          {options.map((option) => (
            <SelectItem key={option.key} value={option.key}>{String(option.option)}</SelectItem>
          ))}
        </SelectInput>
      </Field>
    );
  }

  if (schema.format === "tron-variable-ref") {
    const options = variableOptions.some((option) => option.value === value)
      ? variableOptions
      : value ? [{ label: String(value), value: String(value) }, ...variableOptions] : variableOptions;
    return (
      <Field error={error} hint={hint} label={fieldLabel(label, required)}>
        <div className="relative">
          <KeyRound aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 z-10 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} />
          <SelectInput
            aria-label={label}
            className="w-full pl-9 font-mono text-[12px]"
            disabled={readOnly}
            placeholder={t("jsonSchemaForm.selectVariable", { defaultValue: "Select a Variable" })}
            value={typeof value === "string" ? value : ""}
            onValueChange={(next) => onChange(next)}
          >
            {options.map((option) => (
              <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
            ))}
          </SelectInput>
        </div>
      </Field>
    );
  }

  if (type === "boolean") {
    return (
      <div className="flex min-h-[68px] items-end pb-1">
        <ToggleSwitch
          ariaLabel={label}
          checked={value === true}
          disabled={readOnly}
          label={label}
          onCheckedChange={onChange}
        />
      </div>
    );
  }

  if (type === "number" || type === "integer") {
    return (
      <Field error={error} hint={hint} label={fieldLabel(label, required)}>
        <NumericValueInput value={value} onChange={onChange} integer={type === "integer"}
          label={label} readOnly={readOnly} placeholder={schema.default === undefined ? undefined : String(schema.default)} />
      </Field>
    );
  }

  if (type === "null") {
    return (
      <Field label={fieldLabel(label, required)}>
        <div className="flex h-10 items-center rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-3 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
          null
        </div>
      </Field>
    );
  }

  const multiline = (schema.maxLength ?? 0) > 500 || label.toLowerCase().includes("description");
  return (
    <Field error={error} hint={hint} label={fieldLabel(label, required)}>
      {multiline ? (
        <TextArea
          aria-label={label}
          className="min-h-20"
          disabled={readOnly}
          maxLength={schema.maxLength}
          minLength={schema.minLength}
          value={typeof value === "string" ? value : ""}
          onChange={(event) => onChange(event.target.value)}
        />
      ) : (
        <TextInput
          aria-label={label}
          disabled={readOnly}
          maxLength={schema.maxLength}
          minLength={schema.minLength}
          value={typeof value === "string" ? value : ""}
          onChange={(event) => onChange(event.target.value)}
        />
      )}
    </Field>
  );
}

function AdditionalPropertiesEditor({
  errors,
  label,
  onChange,
  path,
  readOnly,
  schema,
  value,
  variableOptions,
}: Omit<ValueEditorProps, "depth" | "required" | "schemaValue"> & { schema: JsonSchemaDraft7 }) {
  const { t } = useTranslation("common");
  const objectValue = value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {};
  const valueSchema: JsonSchemaDraft7Value = typeof schema.additionalProperties === "object"
    ? schema.additionalProperties
    : { type: "string" };
  const entries = Object.entries(objectValue);

  function rename(previousName: string, nextName: string) {
    if (!nextName || previousName === nextName || nextName in objectValue) return;
    onChange(Object.fromEntries(entries.map(([name, entryValue]) => (
      name === previousName ? [nextName, entryValue] : [name, entryValue]
    ))));
  }

  function addProperty() {
    let index = entries.length + 1;
    while (`property${index}` in objectValue) index += 1;
    onChange({ ...objectValue, [`property${index}`]: neutralValue(valueSchema) });
  }

  return (
    <section className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface)]">
      <div className="flex min-h-11 items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-4 py-2">
        <div className="flex min-w-0 items-center gap-[12px]">
          <Braces aria-hidden="true" className="shrink-0 text-[var(--color-on-surface-variant)]" size={15} />
          <h3 className="truncate text-sm font-medium">{label}</h3>
          <span className="rounded-full bg-[var(--color-surface-container)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-on-surface-variant)]">{entries.length}</span>
        </div>
        {!readOnly ? (
          <Button size="sm" variant="ghost" onClick={addProperty}>

            {t("jsonSchemaForm.addProperty", { defaultValue: "Add property" })}
          </Button>
        ) : null}
      </div>
      <div className="space-y-2 p-3">
        {entries.map(([name, entryValue]) => (
          <div className="grid min-w-0 gap-[12px] rounded-md border border-[var(--color-outline-variant)] p-2 sm:grid-cols-[minmax(120px,0.7fr)_minmax(180px,1.3fr)_32px] sm:items-start" key={name}>
            <TextInput
              aria-label={t("jsonSchemaForm.propertyName", { defaultValue: "Property name" })}
              className="h-9 font-mono text-[12px]"
              disabled={readOnly}
              defaultValue={name}
              onBlur={(event) => rename(name, event.target.value.trim())}
            />
            <ValueEditor
              depth={1}
              errors={errors}
              label={name}
              path={propertyPath(path, name)}
              readOnly={readOnly}
              required={false}
              schemaValue={valueSchema}
              value={entryValue}
              variableOptions={variableOptions}
              onChange={(nextValue) => onChange({ ...objectValue, [name]: nextValue })}
            />
            {!readOnly ? (
              <Button
                aria-label={t("jsonSchemaForm.removeProperty", { defaultValue: "Remove property" })}
                className="mt-0.5"
                size="icon-sm"
                variant="ghost"
                onClick={() => onChange(Object.fromEntries(entries.filter(([entryName]) => entryName !== name)))}
              >
                <Trash2 aria-hidden="true" size={14} />
              </Button>
            ) : null}
          </div>
        ))}
        {!entries.length ? (
          <p className="px-1 py-2 text-[12px] text-[var(--color-on-surface-variant)]">
            {t("jsonSchemaForm.noProperties", { defaultValue: "No properties" })}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ObjectValueEditor({
  depth,
  errors,
  label,
  onChange,
  path,
  readOnly,
  required,
  schema,
  value,
  variableOptions,
}: Omit<ValueEditorProps, "schemaValue"> & { schema: JsonSchemaDraft7 }) {
  const objectValue = value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {};
  const properties = Object.entries(schema.properties ?? {});

  if (!properties.length && schema.additionalProperties !== false) {
    return (
      <AdditionalPropertiesEditor
        errors={errors}
        label={label}
        path={path}
        readOnly={readOnly}
        schema={schema}
        value={objectValue}
        variableOptions={variableOptions}
        onChange={onChange}
      />
    );
  }

  const fields = (
    <div className="grid min-w-0 gap-x-4 gap-y-4 md:grid-cols-2">
      {properties.map(([name, propertySchema]) => {
        const childSchema = asSchemaObject(propertySchema);
        const childType = inferredType(childSchema, objectValue[name]);
        const wide = childType === "array" || childType === "object";
        return (
          <div className={cn("min-w-0", wide && "md:col-span-2")} key={name}>
            <ValueEditor
              depth={depth + 1}
              errors={errors}
              label={childSchema.title || name}
              path={propertyPath(path, name)}
              readOnly={readOnly}
              required={(schema.required ?? []).includes(name)}
              schemaValue={propertySchema}
              value={Object.hasOwn(objectValue, name) ? objectValue[name] : neutralValue(propertySchema)}
              variableOptions={variableOptions}
              onChange={(nextValue) => onChange({ ...objectValue, [name]: nextValue })}
            />
          </div>
        );
      })}
    </div>
  );

  if (depth === 0) return fields;
  return (
    <section className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
      <div className="border-b border-[var(--color-outline-variant)] px-4 py-2.5">
        <h3 className="text-sm font-medium">{fieldLabel(label, required)}</h3>
        {schema.description ? <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{schema.description}</p> : null}
      </div>
      <div className="p-4">{fields}</div>
    </section>
  );
}

function ArrayValueEditor({
  depth,
  errors,
  label,
  onChange,
  path,
  readOnly,
  required,
  schema,
  value,
  variableOptions,
}: Omit<ValueEditorProps, "schemaValue"> & { schema: JsonSchemaDraft7 }) {
  const { t } = useTranslation("common");
  const arrayValue = Array.isArray(value) ? value : [];
  const itemSchema = !schema.items || Array.isArray(schema.items) ? { type: "string" } as JsonSchemaDraft7 : schema.items;
  const maxReached = schema.maxItems !== undefined && arrayValue.length >= schema.maxItems;

  return (
    <section className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
      <div className="flex min-h-11 items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-4 py-2">
        <div className="min-w-0">
          <div className="flex items-center gap-[12px]">
            <h3 className="truncate text-sm font-medium">{fieldLabel(label, required)}</h3>
            <span className="rounded-full bg-[var(--color-surface-container)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-on-surface-variant)]">{arrayValue.length}</span>
          </div>
          {schema.description ? <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{schema.description}</p> : null}
        </div>
        {!readOnly ? (
          <Button
            disabled={maxReached}
            size="sm"
            variant="ghost"
            onClick={() => onChange([...arrayValue, neutralValue(itemSchema)])}
          >

            {t("jsonSchemaForm.addItem", { defaultValue: "Add item" })}
          </Button>
        ) : null}
      </div>
      <div className="space-y-2 p-3">
        {arrayValue.map((item, index) => {
          const itemType = inferredType(asSchemaObject(itemSchema), item);
          const complexItem = itemType === "array" || itemType === "object";
          return (
            <div className={cn("flex min-w-0 gap-[12px] rounded-md border border-[var(--color-outline-variant)] p-2.5", complexItem ? "items-start" : "items-center")} key={index}>
              <span className={cn("w-5 shrink-0 text-right font-mono text-[11px] text-[var(--color-on-surface-variant)]", complexItem && "mt-2")}>{index + 1}</span>
              <div className={cn(
                "min-w-0 flex-1",
                !complexItem && "[&_[data-slot=field-label]]:sr-only [&_[data-slot=field]]:gap-0",
              )}>
                <ValueEditor
                  depth={itemType === "object" ? 0 : depth + 1}
                  errors={errors}
                  label={t("jsonSchemaForm.item", { defaultValue: "Item {{index}}", index: index + 1 })}
                  path={itemPath(path, index)}
                  readOnly={readOnly}
                  required
                  schemaValue={itemSchema}
                  value={item}
                  variableOptions={variableOptions}
                  onChange={(nextValue) => onChange(arrayValue.map((entry, entryIndex) => entryIndex === index ? nextValue : entry))}
                />
              </div>
              {!readOnly ? (
                <Button
                  aria-label={t("jsonSchemaForm.removeItem", { defaultValue: "Remove item {{index}}", index: index + 1 })}
                  disabled={schema.minItems !== undefined && arrayValue.length <= schema.minItems}
                  size="icon-sm"
                  variant="ghost"
                  onClick={() => onChange(arrayValue.filter((_, entryIndex) => entryIndex !== index))}
                >
                  <Trash2 aria-hidden="true" size={14} />
                </Button>
              ) : null}
            </div>
          );
        })}
        {!arrayValue.length ? (
          <p className="px-1 py-2 text-[12px] text-[var(--color-on-surface-variant)]">
            {t("jsonSchemaForm.noItems", { defaultValue: "No items" })}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ValueEditor(props: ValueEditorProps) {
  const schema = asSchemaObject(props.schemaValue);
  const types = declaredTypes(schema);
  const type = inferredType(schema, props.value);

  if (types.length > 1) {
    return (
      <div className="grid min-w-0 gap-[12px] sm:grid-cols-[112px_minmax(0,1fr)]">
        <SelectInput
          aria-label={`${props.label} type`}
          className="w-full font-mono text-[12px]"
          disabled={props.readOnly}
          size="sm"
          value={type}
          onValueChange={(nextType) => props.onChange(neutralValue({ ...schema, type: nextType as JsonSchemaDraft7Type }))}
        >
          {types.map((option) => <SelectItem key={option} value={option}>{option}</SelectItem>)}
        </SelectInput>
        <ValueEditor {...props} schemaValue={{ ...schema, type }} />
      </div>
    );
  }

  if (type === "object") return <ObjectValueEditor {...props} schema={schema} />;
  if (type === "array") return <ArrayValueEditor {...props} schema={schema} />;
  return <PrimitiveValueEditor {...props} schema={schema} type={type} />;
}

export function JsonSchemaDraft7Form({
  className,
  errors = [],
  onChange,
  readOnly = false,
  schema,
  value,
  variableOptions = defaultVariableOptions,
}: JsonSchemaDraft7FormProps) {
  const normalizedVariables = useMemo(() => (
    [...variableOptions].sort((left, right) => left.label.localeCompare(right.label))
  ), [variableOptions]);

  return (
    <div className={cn("min-w-0", className)}>
      <ValueEditor
        depth={0}
        errors={errors}
        label={schema.title || "Resource attributes"}
        path="$"
        readOnly={readOnly}
        required
        schemaValue={schema}
        value={value}
        variableOptions={normalizedVariables}
        onChange={(nextValue) => onChange(nextValue as Record<string, unknown>)}
      />
    </div>
  );
}
