import {
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Trash2,
  X,
  AlertCircle,
  Regex,
} from "lucide-react";
import { type ComponentProps, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/shared/lib/utils";

import {
  asSchemaObject,
  createSchemaForType,
  nextPropertyName,
  renameProperty,
  reorderProperty,
  replaceSchemaType,
  schemaEditorType,
} from "./schema";
import type {
  JsonSchemaDraft7,
  JsonSchemaDraft7EditorType,
  JsonSchemaDraft7FormatOption,
  JsonSchemaDraft7RootObject,
  JsonSchemaDraft7Value,
} from "./types";

/*
 * Adapted from JSONJoy Builder's SchemaPropertyEditor, ObjectEditor,
 * ArrayEditor, and type-editor layout. See NOTICE.md for MIT attribution.
 * Project primitives, tokens, localization, types, and state flow are Tron-native.
 */

const editorTypes: JsonSchemaDraft7EditorType[] = [
  "string",
  "integer",
  "number",
  "boolean",
  "array",
  "object",
  "null",
  "anyOf",
  "oneOf",
  "allOf",
];

interface JsonSchemaDraft7VisualEditorProps {
  formatOptions: JsonSchemaDraft7FormatOption[];
  onChange: (schema: JsonSchemaDraft7RootObject) => void;
  readOnly: boolean;
  schema: JsonSchemaDraft7RootObject;
}

interface SchemaNodeEditorProps {
  depth: number;
  formatOptions: JsonSchemaDraft7FormatOption[];
  onChange: (schema: JsonSchemaDraft7) => void;
  readOnly: boolean;
  schema: JsonSchemaDraft7Value;
}

interface PrimitiveConstraintsProps {
  onChange: (schema: JsonSchemaDraft7) => void;
  readOnly: boolean;
  schema: JsonSchemaDraft7;
}

function removeUndefinedKeyword(schema: JsonSchemaDraft7, keyword: string, value: unknown) {
  const next = { ...schema };
  if (value === undefined || value === "") delete next[keyword];
  else next[keyword] = value;
  return next;
}

function inputNumber(value: string) {
  return value === "" ? undefined : Number(value);
}

function typeTone(_type: JsonSchemaDraft7EditorType) {
  return "border-border bg-muted text-foreground";
}

function SchemaTypeSelect({
  compact = false,
  onChange,
  readOnly,
  schema,
}: {
  compact?: boolean;
  onChange: (type: JsonSchemaDraft7EditorType) => void;
  readOnly: boolean;
  schema: JsonSchemaDraft7Value;
}) {
  const { t } = useTranslation("common");
  const type = schemaEditorType(schema);
  const label = (value: JsonSchemaDraft7EditorType) => t(`jsonSchemaEditor.types.${value}`, { defaultValue: value });
  return (
    <SelectInput
      aria-label={t("jsonSchemaEditor.fields.type", { defaultValue: "Type" })}
      className={cn(
        "w-full",
        compact && "h-8 w-[116px] rounded-xl border px-3 text-[12px] font-medium shadow-none",
        compact && typeTone(type),
      )}
      disabled={readOnly}
      size={compact ? "sm" : "lg"}
      value={type}
      onValueChange={(value) => onChange(value as JsonSchemaDraft7EditorType)}
    >
      {editorTypes.map((item) => <SelectItem key={item} value={item}>{label(item)}</SelectItem>)}
    </SelectInput>
  );
}

function DenseNumberInput({ value, onChange, ...props }: ComponentProps<typeof TextInput>) {
  const [draft, setDraft] = useState(value == null ? "" : String(value));
  useEffect(() => {
    const next = value == null ? "" : String(value);
    setDraft((current) => next === "" && ["-", ".", "-."].includes(current) ? current : current && next !== "" && Number(current) === Number(next) ? current : next);
  }, [value]);
  return <TextInput {...props} type="text" inputMode={props.step === "any" ? "decimal" : "numeric"} value={draft} onChange={(event) => {
    const raw = event.currentTarget.value;
    const pattern = props.step === "any" ? /^-?\d*(?:\.\d*)?$/ : /^\d*$/;
    if (!pattern.test(raw)) return;
    setDraft(raw);
    if (raw === "-" || raw === "." || raw === "-.") {
      event.currentTarget.value = "";
      onChange?.(event);
      event.currentTarget.value = raw;
    } else onChange?.(event);
  }} />;
}

function DenseTextInput(props: ComponentProps<typeof TextInput>) {
  const className = cn("h-9 text-[13px]", props.className);
  return props.type === "number" ? <DenseNumberInput {...props} className={className} /> : <TextInput {...props} className={className} />;
}

function StringConstraints({ formatOptions, onChange, readOnly, schema }: PrimitiveConstraintsProps & { formatOptions: JsonSchemaDraft7FormatOption[] }) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback });
  const enumValue = Array.isArray(schema.enum) ? schema.enum.filter((item): item is string => typeof item === "string").join(", ") : "";
  return (
    <div className="grid gap-3 md:grid-cols-2">
      <Field label={tr("fields.format", "Format")}>
        <SelectInput className="h-9 w-full" disabled={readOnly} size="sm" value={schema.format || "none"} onValueChange={(value) => onChange(removeUndefinedKeyword(schema, "format", value === "none" ? undefined : value))}>
          <SelectItem value="none">{tr("format.none", "None")}</SelectItem>
          {formatOptions.map((format) => <SelectItem key={format.value} value={format.value}>{format.label ?? format.value}</SelectItem>)}
        </SelectInput>
      </Field>
      <Field label={tr("fields.enum", "Allowed values")}>
        <DenseTextInput disabled={readOnly} placeholder={tr("fields.enumPlaceholder", "Comma-separated values")} value={enumValue} onChange={(event) => onChange(removeUndefinedKeyword(schema, "enum", event.target.value.trim() ? event.target.value.split(",").map((item) => item.trim()).filter(Boolean) : undefined))} />
      </Field>
      <Field label={tr("fields.minLength", "Minimum length")}>
        <DenseTextInput min={0} type="number" disabled={readOnly} placeholder={tr("placeholders.noMinimum", "No minimum")} value={schema.minLength ?? ""} onChange={(event) => onChange(removeUndefinedKeyword(schema, "minLength", inputNumber(event.target.value)))} />
      </Field>
      <Field label={tr("fields.maxLength", "Maximum length")}>
        <DenseTextInput min={0} type="number" disabled={readOnly} placeholder={tr("placeholders.noMaximum", "No maximum")} value={schema.maxLength ?? ""} onChange={(event) => onChange(removeUndefinedKeyword(schema, "maxLength", inputNumber(event.target.value)))} />
      </Field>
      <div className="md:col-span-2">
        <Field label={tr("fields.pattern", "Pattern")}>
          <DenseTextInput className="font-mono text-[12px]" disabled={readOnly} value={schema.pattern ?? ""} onChange={(event) => onChange(removeUndefinedKeyword(schema, "pattern", event.target.value || undefined))} />
        </Field>
      </div>
    </div>
  );
}

function NumberConstraints({ onChange, readOnly, schema }: PrimitiveConstraintsProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback });
  const enumValue = Array.isArray(schema.enum) ? schema.enum.filter((item): item is number => typeof item === "number").join(", ") : "";
  const numberField = (keyword: "exclusiveMaximum" | "exclusiveMinimum" | "maximum" | "minimum" | "multipleOf", label: string, min?: number) => (
    <Field label={label}>
      <DenseTextInput min={min} step="any" type="number" disabled={readOnly} value={schema[keyword] ?? ""} onChange={(event) => onChange(removeUndefinedKeyword(schema, keyword, inputNumber(event.target.value)))} />
    </Field>
  );
  return (
    <div className="grid gap-3 md:grid-cols-2">
      {numberField("minimum", tr("fields.minimum", "Minimum"))}
      {numberField("maximum", tr("fields.maximum", "Maximum"))}
      {numberField("exclusiveMinimum", tr("fields.exclusiveMinimum", "Exclusive minimum"))}
      {numberField("exclusiveMaximum", tr("fields.exclusiveMaximum", "Exclusive maximum"))}
      {numberField("multipleOf", tr("fields.multipleOf", "Multiple of"), 0)}
      <Field label={tr("fields.enum", "Allowed values")}>
        <DenseTextInput disabled={readOnly} placeholder={tr("fields.enumPlaceholder", "Comma-separated values")} value={enumValue} onChange={(event) => {
          const values = event.target.value.split(",").map((item) => Number(item.trim())).filter((item) => Number.isFinite(item));
          onChange(removeUndefinedKeyword(schema, "enum", event.target.value.trim() ? values : undefined));
        }} />
      </Field>
    </div>
  );
}

function BooleanConstraints({ onChange, readOnly, schema }: PrimitiveConstraintsProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback });
  const restriction = !schema.enum ? "any" : schema.enum.length === 1 && schema.enum[0] === true ? "true" : "false";
  return (
    <div className="max-w-sm">
      <Field label={tr("fields.booleanValues", "Allowed boolean values")}>
        <SelectInput className="h-9 w-full" disabled={readOnly} size="sm" value={restriction} onValueChange={(value) => onChange(removeUndefinedKeyword(schema, "enum", value === "any" ? undefined : [value === "true"]))}>
          <SelectItem value="any">{tr("boolean.any", "true and false")}</SelectItem>
          <SelectItem value="true">true</SelectItem>
          <SelectItem value="false">false</SelectItem>
        </SelectInput>
      </Field>
    </div>
  );
}

interface ObjectPropertiesEditorProps {
  depth: number;
  formatOptions: JsonSchemaDraft7FormatOption[];
  onChange: (schema: JsonSchemaDraft7) => void;
  readOnly: boolean;
  root?: boolean;
  schema: JsonSchemaDraft7;
}

interface SchemaPropertyRowProps {
  depth: number;
  formatOptions: JsonSchemaDraft7FormatOption[];
  index: number;
  name: string;
  onMove: (direction: -1 | 1) => void;
  onRemove: () => void;
  onRename: (nextName: string) => void;
  onRequiredChange: (required: boolean) => void;
  onSchemaChange: (schema: JsonSchemaDraft7) => void;
  pattern: boolean;
  readOnly: boolean;
  required: boolean;
  siblingCount: number;
  value: JsonSchemaDraft7Value;
}

function SchemaPropertyRow({
  depth,
  formatOptions,
  index,
  name,
  onMove,
  onRemove,
  onRename,
  onRequiredChange,
  onSchemaChange,
  pattern,
  readOnly,
  required,
  siblingCount,
  value,
}: SchemaPropertyRowProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string, options?: Record<string, unknown>) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback, ...options });
  const [expanded, setExpanded] = useState(false);
  const [editingName, setEditingName] = useState(false);
  const [editingTitle, setEditingTitle] = useState(false);
  const schema = asSchemaObject(value);
  const summary = schema.title || schema.description || tr("fields.addDescription", "Add description");

  return (
    <section className={cn(
      "group mb-2 overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] transition-colors",
      depth > 0 && "ml-2 sm:ml-4",
      expanded && "border-[var(--color-outline)]",
    )}
    >
      <div className="flex min-h-12 min-w-0 flex-wrap items-center gap-[12px] px-3 py-2 sm:flex-nowrap">
        <button aria-label={expanded ? tr("actions.collapse", "Collapse {{code}}", { code: name }) : tr("actions.expand", "Expand {{code}}", { code: name })} className="shrink-0 text-[var(--color-on-surface-variant)]" type="button" onClick={() => setExpanded((current) => !current)}>
          {expanded ? <ChevronDown aria-hidden="true" size={17} /> : <ChevronRight aria-hidden="true" size={17} />}
        </button>
        <div className="flex min-w-0 flex-1 basis-[calc(100%-26px)] items-center gap-[12px] sm:basis-auto">
          {editingName && !readOnly ? (
            <DenseTextInput
              aria-label={tr("fields.code", "Field code")}
              autoFocus
              className="w-36 font-mono"
              defaultValue={name}
              onBlur={(event) => { onRename(event.target.value); setEditingName(false); }}
              onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }}
            />
          ) : (
            <button className="max-w-[52%] shrink-0 truncate rounded-md px-1.5 py-1 text-left font-mono text-[13px] font-medium hover:bg-[var(--color-hover-surface)] sm:max-w-[42%]" disabled={readOnly} type="button" onClick={() => setEditingName(true)}>{name}</button>
          )}

          {editingTitle && !readOnly ? (
            <DenseTextInput
              aria-label={tr("fields.title", "Display label")}
              autoFocus
              className="min-w-28 flex-1 italic"
              defaultValue={schema.title ?? ""}
              onBlur={(event) => { onSchemaChange(removeUndefinedKeyword(schema, "title", event.target.value.trim() || undefined)); setEditingTitle(false); }}
              onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }}
            />
          ) : (
            <button
              aria-label={tr("actions.editTitle", "Edit display label for {{code}}", { code: name })}
              className={cn("min-w-0 flex-1 truncate px-1.5 py-1 text-left text-[12px] italic text-[var(--color-on-surface-variant)]", !schema.title && !schema.description && "opacity-50")}
              disabled={readOnly}
              type="button"
              onClick={() => setEditingTitle(true)}
            >
              {summary}
            </button>
          )}
        </div>

        <div className="ml-6 flex w-[calc(100%-24px)] shrink-0 items-center justify-end gap-[12px] sm:ml-0 sm:w-auto">
          <SchemaTypeSelect compact readOnly={readOnly} schema={value} onChange={(type) => onSchemaChange(replaceSchemaType(value, type))} />
          {pattern ? (
            <span className="inline-flex h-8 min-w-[84px] items-center justify-center gap-1 rounded-xl border border-border bg-muted px-3 text-[12px] font-medium text-muted-foreground"><Regex aria-hidden="true" size={12} />regex</span>
          ) : (
            <button
              aria-pressed={required}
              className={cn("h-8 min-w-[84px] rounded-xl px-3 text-[12px] font-medium", required ? "bg-destructive/10 text-destructive-foreground" : "bg-muted text-muted-foreground")}
              disabled={readOnly}
              type="button"
              onClick={() => onRequiredChange(!required)}
            >
              {required ? tr("required", "Required") : tr("optional", "Optional")}
            </button>
          )}
          {!readOnly ? (
            <Button
              aria-label={tr("actions.remove", "Remove {{code}}", { code: name })}
              className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
              size="icon-sm"
              variant="ghost"
              onClick={onRemove}
            >
              <X aria-hidden="true" size={15} />
            </Button>
          ) : null}
        </div>
      </div>

      {expanded ? (
        <div className="space-y-3 border-t border-[var(--color-outline-variant)] px-4 py-3">
          <Field label={tr("fields.description", "Description")}>
            <DenseTextInput disabled={readOnly} placeholder={tr("fields.descriptionPlaceholder", "Explain this field") } value={schema.description ?? ""} onChange={(event) => onSchemaChange(removeUndefinedKeyword(schema, "description", event.target.value || undefined))} />
          </Field>
          {schema.format === "tron-variable-ref" ? <Badge variant="warning">tron-variable-ref</Badge> : null}
          <SchemaNodeEditor depth={depth + 1} formatOptions={formatOptions} readOnly={readOnly} schema={value} onChange={onSchemaChange} />
          {!readOnly ? <div className="flex justify-end gap-1">
            <Button aria-label={tr("actions.moveUp", "Move {{code}} up", { code: name })} disabled={readOnly || index === 0} size="icon-sm" variant="ghost" onClick={() => onMove(-1)}><ChevronUp aria-hidden="true" size={13} /></Button>
            <Button aria-label={tr("actions.moveDown", "Move {{code}} down", { code: name })} disabled={readOnly || index === siblingCount - 1} size="icon-sm" variant="ghost" onClick={() => onMove(1)}><ChevronDown aria-hidden="true" size={13} /></Button>
          </div> : null}
        </div>
      ) : null}
    </section>
  );
}

function ObjectPropertiesEditor({ depth, formatOptions, onChange, readOnly, root = false, schema }: ObjectPropertiesEditorProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string, options?: Record<string, unknown>) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback, ...options });
  const [propertyError, setPropertyError] = useState("");
  const properties = schema.properties ?? {};
  const patternProperties = schema.patternProperties ?? {};
  const required = schema.required ?? [];
  const entries = [
    ...Object.entries(properties).map(([name, value]) => ({ name, pattern: false, value })),
    ...Object.entries(patternProperties).map(([name, value]) => ({ name, pattern: true, value })),
  ];

  function updateCollections(nextProperties: Record<string, JsonSchemaDraft7Value>, nextRequired = required, nextPatterns = patternProperties) {
    const next = { ...schema, properties: nextProperties, required: nextRequired };
    if (Object.keys(nextPatterns).length) next.patternProperties = nextPatterns;
    else delete next.patternProperties;
    onChange(next);
  }

  function addProperty(pattern = false) {
    const collection = pattern ? patternProperties : properties;
    const name = nextPropertyName(collection, pattern ? "pattern" : "field");
    const next: Record<string, JsonSchemaDraft7Value> = { ...collection, [name]: { description: "", type: "string" } };
    updateCollections(pattern ? properties : next, required, pattern ? next : patternProperties);
  }

  function renameEntry(previousName: string, nextValue: string, pattern: boolean) {
    const nextName = nextValue.trim();
    const collection = pattern ? patternProperties : properties;
    if (!nextName || nextName === previousName) return;
    if (!pattern && !/^[A-Za-z_][A-Za-z0-9_]*$/.test(nextName)) {
      setPropertyError(tr("errors.propertyCode", "Field codes must start with a letter or underscore and contain only letters, numbers, or underscores."));
      return;
    }
    if (pattern) {
      try { new RegExp(nextName); } catch {
        setPropertyError(tr("errors.patternCode", "Pattern property names must be valid regular expressions."));
        return;
      }
    }
    if (collection[nextName]) {
      setPropertyError(tr("errors.duplicateCode", "Field code already exists."));
      return;
    }
    const renamed = renameProperty(collection, previousName, nextName);
    updateCollections(pattern ? properties : renamed, pattern ? required : required.map((name) => name === previousName ? nextName : name), pattern ? renamed : patternProperties);
    setPropertyError("");
  }

  function updateEntry(name: string, value: JsonSchemaDraft7, pattern: boolean) {
    const collection = pattern ? patternProperties : properties;
    const next = { ...collection, [name]: value };
    updateCollections(pattern ? properties : next, required, pattern ? next : patternProperties);
  }

  function removeEntry(name: string, pattern: boolean) {
    const collection = { ...(pattern ? patternProperties : properties) };
    delete collection[name];
    updateCollections(pattern ? properties : collection, pattern ? required : required.filter((item) => item !== name), pattern ? collection : patternProperties);
  }

  function moveEntry(name: string, direction: -1 | 1, pattern: boolean) {
    const collection = pattern ? patternProperties : properties;
    const reordered = reorderProperty(collection, name, direction);
    updateCollections(pattern ? properties : reordered, required, pattern ? reordered : patternProperties);
  }

  const additionalProperties = schema.additionalProperties !== false;
  const objectFooter = (
    <div className={cn("flex flex-wrap items-center gap-[12px]", !root && "pt-1")}>
      <Button disabled={readOnly} size="sm" variant={root ? "default" : "outline"} onClick={() => addProperty(false)}>{tr("actions.add", "Add field")}</Button>
      <Button disabled={readOnly} size="sm" variant="ghost" onClick={() => addProperty(true)}>{tr("actions.addPattern", "Add pattern")}</Button>
      <button
        aria-pressed={additionalProperties}
        className={cn("h-8 rounded-xl px-3 text-[12px] font-medium", additionalProperties ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground")}
        disabled={readOnly}
        type="button"
        onClick={() => onChange({ ...schema, additionalProperties: !additionalProperties })}
      >
        {additionalProperties ? tr("root.additionalAllowed", "Allow additional properties") : tr("root.additionalForbidden", "Forbid additional properties")}
      </button>
      {root ? <span className="ml-auto text-[11px] text-[var(--color-on-surface-variant)]">{tr("root.summary", "{{value}} properties · JSON Schema Draft 7", { value: entries.length })}</span> : null}
    </div>
  );

  return (
    <div className="space-y-2">
      {root ? objectFooter : null}
      {propertyError ? (
        <div className="flex items-start gap-[12px] rounded-lg bg-destructive/10 px-3 py-2 text-[12px] text-destructive-foreground"><AlertCircle aria-hidden="true" className="mt-0.5 shrink-0" size={14} />{propertyError}</div>
      ) : null}
      {entries.length ? entries.map(({ name, pattern, value }) => {
        const sameKindEntries = pattern ? Object.keys(patternProperties) : Object.keys(properties);
        const sameKindIndex = sameKindEntries.indexOf(name);
        return (
          <SchemaPropertyRow
            depth={depth}
            formatOptions={formatOptions}
            index={sameKindIndex}
            key={`${pattern ? "pattern:" : ""}${name}`}
            name={name}
            pattern={pattern}
            readOnly={readOnly}
            required={!pattern && required.includes(name)}
            siblingCount={sameKindEntries.length}
            value={value}
            onMove={(direction) => moveEntry(name, direction, pattern)}
            onRemove={() => removeEntry(name, pattern)}
            onRename={(nextName) => renameEntry(name, nextName, pattern)}
            onRequiredChange={(nextRequired) => updateCollections(properties, nextRequired ? Array.from(new Set([...required, name])) : required.filter((item) => item !== name), patternProperties)}
            onSchemaChange={(nextSchema) => updateEntry(name, nextSchema, pattern)}
          />
        );
      }) : (
        <div className="rounded-xl border border-dashed border-[var(--color-outline-variant)] px-4 py-6 text-center text-[12px] text-[var(--color-on-surface-variant)]">{tr("empty.description", "Add the first property to define this object.")}</div>
      )}
      {!root ? objectFooter : null}
    </div>
  );
}

function ObjectConstraints(props: SchemaNodeEditorProps) {
  const schema = asSchemaObject(props.schema);
  return <ObjectPropertiesEditor depth={props.depth} formatOptions={props.formatOptions} readOnly={props.readOnly} schema={{ ...schema, properties: schema.properties ?? {}, required: schema.required ?? [], type: "object" }} onChange={props.onChange} />;
}

function ArrayConstraints(props: SchemaNodeEditorProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback });
  const schema = asSchemaObject(props.schema);
  const tupleItems = Array.isArray(schema.items);
  const items: JsonSchemaDraft7Value | null = Array.isArray(schema.items)
    ? null
    : schema.items ?? { type: "string" };
  return (
    <div className="space-y-3">
      <div className="grid gap-3 md:grid-cols-2">
        <Field label={tr("fields.minItems", "Minimum items")}><DenseTextInput min={0} type="number" disabled={props.readOnly} placeholder={tr("placeholders.noMinimum", "No minimum")} value={schema.minItems ?? ""} onChange={(event) => props.onChange(removeUndefinedKeyword(schema, "minItems", inputNumber(event.target.value)))} /></Field>
        <Field label={tr("fields.maxItems", "Maximum items")}><DenseTextInput min={0} type="number" disabled={props.readOnly} placeholder={tr("placeholders.noMaximum", "No maximum")} value={schema.maxItems ?? ""} onChange={(event) => props.onChange(removeUndefinedKeyword(schema, "maxItems", inputNumber(event.target.value)))} /></Field>
      </div>
      <button aria-pressed={Boolean(schema.uniqueItems)} className={cn("h-8 rounded-xl px-3 text-[12px] font-medium", schema.uniqueItems ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground")} disabled={props.readOnly} type="button" onClick={() => props.onChange(removeUndefinedKeyword(schema, "uniqueItems", schema.uniqueItems ? undefined : true))}>{schema.uniqueItems ? tr("array.unique", "Unique items") : tr("array.duplicates", "Duplicates allowed")}</button>
      <div className="rounded-xl border border-[var(--color-outline-variant)] px-3 py-2.5">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <p className="text-[13px] font-medium">{tr("array.items", "Item schema")}</p>
            <p className="truncate text-[11px] text-[var(--color-on-surface-variant)]">{tr("array.itemsDescription", "Every array item uses this schema")}</p>
          </div>
          {items ? <SchemaTypeSelect compact readOnly={props.readOnly} schema={items} onChange={(type) => props.onChange({ ...schema, items: replaceSchemaType(items, type) })} /> : null}
        </div>
        {tupleItems ? <div className="mt-2 rounded-lg border border-border bg-muted px-3 py-2 text-[12px] text-muted-foreground">{tr("array.tupleJsonOnly", "Tuple item schemas are preserved in JSON mode.")}</div> : items ? <div className="mt-3 border-t border-[var(--color-outline-variant)] pt-3"><SchemaNodeEditor depth={props.depth + 1} formatOptions={props.formatOptions} readOnly={props.readOnly} schema={items} onChange={(next) => props.onChange({ ...schema, items: next })} /></div> : null}
      </div>
    </div>
  );
}

function CombinatorConstraints({ depth, formatOptions, onChange, readOnly, schema: schemaValue }: SchemaNodeEditorProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string, options?: Record<string, unknown>) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback, ...options });
  const schema = asSchemaObject(schemaValue);
  const combinator = schemaEditorType(schema) as "allOf" | "anyOf" | "oneOf";
  const options = schema[combinator] ?? [];
  const updateOptions = (next: JsonSchemaDraft7Value[]) => onChange({ ...schema, [combinator]: next });
  return (
    <div className="space-y-2">
      <p className="text-[11px] text-[var(--color-on-surface-variant)]">{tr(`combinator.${combinator}`, "Define combined schemas.")}</p>
      {options.map((option, index) => (
        <div className="rounded-xl border border-[var(--color-outline-variant)] p-3" key={`${combinator}-${index}`}>
          <div className="mb-3 flex items-center justify-between gap-3">
            <span className="text-[13px] font-medium">{tr("combinator.option", "Schema {{value}}", { value: index + 1 })}</span>
            <div className="flex items-center gap-1"><SchemaTypeSelect compact readOnly={readOnly} schema={option} onChange={(type) => updateOptions(options.map((item, itemIndex) => itemIndex === index ? replaceSchemaType(item, type) : item))} /><Button aria-label={tr("combinator.remove", "Remove schema {{value}}", { value: index + 1 })} disabled={readOnly} size="icon-sm" variant="ghost" onClick={() => updateOptions(options.filter((_, itemIndex) => itemIndex !== index))}><Trash2 aria-hidden="true" size={14} /></Button></div>
          </div>
          <SchemaNodeEditor depth={depth + 1} formatOptions={formatOptions} readOnly={readOnly} schema={option} onChange={(next) => updateOptions(options.map((item, itemIndex) => itemIndex === index ? next : item))} />
        </div>
      ))}
      <Button disabled={readOnly} size="sm" variant="outline" onClick={() => updateOptions([...options, createSchemaForType("string")])}>{tr("combinator.add", "Add schema")}</Button>
    </div>
  );
}

function SchemaNodeEditor({ depth, formatOptions, onChange, readOnly, schema: schemaValue }: SchemaNodeEditorProps) {
  const schema = asSchemaObject(schemaValue);
  const type = schemaEditorType(schemaValue);
  if (type === "string") return <StringConstraints formatOptions={formatOptions} readOnly={readOnly} schema={schema} onChange={onChange} />;
  if (type === "number" || type === "integer") return <NumberConstraints readOnly={readOnly} schema={schema} onChange={onChange} />;
  if (type === "boolean") return <BooleanConstraints readOnly={readOnly} schema={schema} onChange={onChange} />;
  if (type === "object") return <ObjectConstraints depth={depth} formatOptions={formatOptions} readOnly={readOnly} schema={schema} onChange={onChange} />;
  if (type === "array") return <ArrayConstraints depth={depth} formatOptions={formatOptions} readOnly={readOnly} schema={schema} onChange={onChange} />;
  if (type === "allOf" || type === "anyOf" || type === "oneOf") return <CombinatorConstraints depth={depth} formatOptions={formatOptions} readOnly={readOnly} schema={schema} onChange={onChange} />;
  return null;
}

export function JsonSchemaDraft7VisualEditor({ formatOptions, onChange, readOnly, schema }: JsonSchemaDraft7VisualEditorProps) {
  return (
    <div className="min-w-0 p-4">
      <ObjectPropertiesEditor
        depth={0}
        formatOptions={formatOptions}
        readOnly={readOnly}
        root
        schema={schema}
        onChange={(next) => onChange({
          ...schema,
          ...next,
          $schema: schema.$schema,
          additionalProperties: next.additionalProperties ?? false,
          properties: next.properties ?? {},
          required: next.required ?? [],
          type: "object",
        })}
      />
    </div>
  );
}
