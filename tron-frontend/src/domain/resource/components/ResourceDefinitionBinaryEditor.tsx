import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Database,
  FileKey2,
  LockKeyhole,
} from "lucide-react";

import { SelectItem } from "@/components/ui/select";
import type { ResourceDefinitionBinarySchema } from "@/domain/resource/model";
import {
  Badge,
  Field,
  SelectInput,
  SurfaceCard,
  TextInput,
  ToggleSwitch,
} from "@/shared";

interface ResourceDefinitionBinaryEditorProps {
  onChange: (schema: ResourceDefinitionBinarySchema) => void;
  readOnly: boolean;
  schema: ResourceDefinitionBinarySchema;
}

export function validateResourceDefinitionBinarySchema(schema: ResourceDefinitionBinarySchema) {
  const errors: Partial<Record<keyof ResourceDefinitionBinarySchema, string>> = {};
  if (schema.minCount < 0) errors.minCount = "Minimum count cannot be negative.";
  if (schema.maxCount < schema.minCount) errors.maxCount = "Maximum count must be greater than or equal to minimum count.";
  if (schema.maxCount > 0 && schema.maxSizeBytes <= 0) errors.maxSizeBytes = "Maximum object size must be greater than zero.";
  if (schema.maxCount > 0 && schema.maxNameLength <= 0) errors.maxNameLength = "Maximum name length must be greater than zero.";
  if (schema.maxCount > 0 && schema.namePattern) {
    try {
      new RegExp(schema.namePattern);
    } catch {
      errors.namePattern = "Name pattern must be a valid regular expression.";
    }
  }
  return errors;
}

export function ResourceDefinitionBinaryEditor({ onChange, readOnly, schema }: ResourceDefinitionBinaryEditorProps) {
  const { t } = useTranslation("pages");
  const supportsBinaries = schema.maxCount > 0;
  const errors = useMemo(() => validateResourceDefinitionBinarySchema(schema), [schema]);
  const valid = Object.keys(errors).length === 0;
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`resourceDefinitionEditor.binary.${key}`, { defaultValue, ...options })
  );

  function patch(values: Partial<ResourceDefinitionBinarySchema>) {
    onChange({ ...schema, ...values });
  }

  function toggleSupport(checked: boolean) {
    if (!checked) {
      patch({ minCount: 0, maxCount: 0 });
      return;
    }
    patch({
      maxCount: schema.maxCount || 10,
      maxNameLength: schema.maxNameLength || 255,
      maxSizeBytes: schema.maxSizeBytes || 10485760,
      minCount: Math.max(1, schema.minCount),
      namePattern: schema.namePattern || "^[^/]+$",
    });
  }

  return (
    <SurfaceCard className="overflow-hidden p-0">
      <header className="flex flex-wrap items-center justify-between gap-4 border-b border-[var(--color-outline-variant)] px-5 py-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-base font-semibold">{editorT("title", "Binary constraints")}</h2>
            <Badge tone={valid ? "success" : "error"}>{valid ? editorT("valid", "VALID") : editorT("invalid", "INVALID")}</Badge>
            {readOnly ? <Badge tone="outline">{editorT("readOnly", "READ ONLY")}</Badge> : null}
          </div>
          <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
            {editorT("description", "Define storage routing and protocol limits without interpreting binary contents.")}
          </p>
        </div>
        <ToggleSwitch
          checked={supportsBinaries}
          disabled={readOnly}
          label={editorT("supports", "Supports binaries")}
          onCheckedChange={toggleSupport}
        />
      </header>

      {!supportsBinaries ? (
        <div className="flex min-h-48 flex-col items-center justify-center px-5 py-10 text-center">
          <div className="flex size-11 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
            <FileKey2 aria-hidden="true" size={20} />
          </div>
          <p className="mt-3 text-sm font-medium">{editorT("disabled.title", "This definition does not accept binaries")}</p>
          <code className="mt-2 rounded-lg bg-[var(--color-surface-container-low)] px-3 py-1.5 text-[12px] text-[var(--color-on-surface-variant)]">
            {`{ "minCount": 0, "maxCount": 0 }`}
          </code>
        </div>
      ) : (
        <div className="max-w-5xl space-y-6 p-5">
          <section className="grid gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <div className="flex items-center gap-2 text-sm font-semibold">
                <Database aria-hidden="true" size={16} />
                {editorT("storage.title", "Storage profile")}
              </div>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">
                {editorT("storage.description", "Select a configured tron.yaml profile. Resolved paths and credentials stay hidden.")}
              </p>
            </div>
            <Field label={editorT("storage.field", "Profile")}>
              <SelectInput
                className="w-full"
                disabled={readOnly}
                value={schema.storageProfile}
                onValueChange={(value) => {
                  if (value) patch({ storageProfile: value });
                }}
              >
                <SelectItem value="@default">@default</SelectItem>
                <SelectItem value="media-local">media-local</SelectItem>
                <SelectItem value="documents-local">documents-local</SelectItem>
              </SelectInput>
            </Field>
          </section>

          <section className="grid gap-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("count.title", "Object count")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("count.description", "Minimum and maximum binaries mapped to one Resource version.")}</p>
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field error={errors.minCount ? editorT("errors.minCount", errors.minCount) : undefined} label={editorT("count.minimum", "Minimum count")}>
                <TextInput disabled={readOnly} min={0} type="number" value={schema.minCount} onChange={(event) => patch({ minCount: Number(event.target.value) })} />
              </Field>
              <Field error={errors.maxCount ? editorT("errors.maxCount", errors.maxCount) : undefined} label={editorT("count.maximum", "Maximum count")}>
                <TextInput disabled={readOnly} min={0} type="number" value={schema.maxCount} onChange={(event) => patch({ maxCount: Number(event.target.value) })} />
              </Field>
            </div>
          </section>

          <section className="grid gap-5 border-t border-[var(--color-outline-variant)] pt-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("size.title", "Per-object size")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("size.description", "Maximum number of bytes accepted for each uploaded object.")}</p>
            </div>
            <Field error={errors.maxSizeBytes ? editorT("errors.maxSizeBytes", errors.maxSizeBytes) : undefined} label={editorT("size.maximum", "Maximum size (bytes)")}>
              <TextInput disabled={readOnly} min={1} type="number" value={schema.maxSizeBytes} onChange={(event) => patch({ maxSizeBytes: Number(event.target.value) })} />
            </Field>
          </section>

          <section className="grid gap-5 border-t border-[var(--color-outline-variant)] pt-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("name.title", "Binary names")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("name.description", "Constrain caller-provided binary names while keeping their business meaning opaque.")}</p>
            </div>
            <div className="grid gap-4">
              <Field error={errors.maxNameLength ? editorT("errors.maxNameLength", errors.maxNameLength) : undefined} label={editorT("name.maxLength", "Maximum name length")}>
                <TextInput disabled={readOnly} min={1} type="number" value={schema.maxNameLength} onChange={(event) => patch({ maxNameLength: Number(event.target.value) })} />
              </Field>
              <Field error={errors.namePattern ? editorT("errors.namePattern", errors.namePattern) : undefined} label={editorT("name.pattern", "Name pattern")}>
                <TextInput className="font-mono" disabled={readOnly} value={schema.namePattern} onChange={(event) => patch({ namePattern: event.target.value })} />
              </Field>
              <div className="flex items-start gap-3 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3">
                <LockKeyhole aria-hidden="true" className="mt-0.5 shrink-0 text-[var(--color-on-surface-variant)]" size={16} />
                <div>
                  <p className="text-sm font-medium">{editorT("name.unique", "Names are unique within one Resource version")}</p>
                  <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{editorT("name.uniqueDescription", "This protocol invariant is always enforced and cannot be disabled.")}</p>
                </div>
              </div>
            </div>
          </section>
        </div>
      )}
    </SurfaceCard>
  );
}
