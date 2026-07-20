import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Database,
  FileKey2,
  LockKeyhole,
} from "lucide-react";

import type { ResourceDefinitionBinarySchema } from "@/domain/resource/model";
import {
  Badge,
  Field,
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
  if (schema.minCount < 0) errors.minCount = "Minimum file count cannot be negative.";
  if (schema.maxCount < schema.minCount) errors.maxCount = "Maximum file count must be greater than or equal to minimum file count.";
  if (schema.maxCount > 0 && schema.maxSizeBytes <= 0) errors.maxSizeBytes = "Maximum file size must be greater than zero.";
  if (schema.maxCount > 0 && schema.maxNameLength <= 0) errors.maxNameLength = "Maximum file name length must be greater than zero.";
  if (schema.maxCount > 0 && schema.namePattern) {
    try {
      new RegExp(schema.namePattern);
    } catch {
      errors.namePattern = "File name rule must be a valid regular expression.";
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
            <h2 className="text-base font-semibold">{editorT("title", "File rules")}</h2>
            <Badge tone={valid ? "success" : "error"}>{valid ? editorT("valid", "VALID") : editorT("invalid", "INVALID")}</Badge>
            {readOnly ? <Badge tone="outline">{editorT("readOnly", "READ ONLY")}</Badge> : null}
          </div>
          <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
            {editorT("description", "Set the number, size, and storage location for uploaded files.")}
          </p>
        </div>
        <ToggleSwitch
          checked={supportsBinaries}
          disabled={readOnly}
          label={editorT("supports", "Allow file uploads")}
          onCheckedChange={toggleSupport}
        />
      </header>

      {!supportsBinaries ? (
        <div className="flex min-h-48 flex-col items-center justify-center px-5 py-10 text-center">
          <div className="flex size-11 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
            <FileKey2 aria-hidden="true" size={20} />
          </div>
          <p className="mt-3 text-sm font-medium">{editorT("disabled.title", "This resource type does not allow file uploads")}</p>
        </div>
      ) : (
        <div className="max-w-5xl space-y-6 p-5">
          <section className="grid gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <div className="flex items-center gap-2 text-sm font-semibold">
                <Database aria-hidden="true" size={16} />
                {editorT("storage.title", "File storage location")}
              </div>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">
                {editorT("storage.description", "Enter the name of a configured storage location. Leave it empty to use the default location.")}
              </p>
            </div>
            <Field label={editorT("storage.field", "Storage name")}>
              <TextInput
                className="w-full"
                disabled={readOnly}
                placeholder="@default"
                value={schema.storageProfile}
                onChange={(event) => patch({ storageProfile: event.target.value })}
              />
            </Field>
          </section>

          <section className="grid gap-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("count.title", "File count")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("count.description", "Set the minimum and maximum number of files for each resource.")}</p>
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field error={errors.minCount ? editorT("errors.minCount", errors.minCount) : undefined} label={editorT("count.minimum", "Minimum files")}>
                <TextInput disabled={readOnly} min={0} type="number" value={schema.minCount} onChange={(event) => patch({ minCount: Number(event.target.value) })} />
              </Field>
              <Field error={errors.maxCount ? editorT("errors.maxCount", errors.maxCount) : undefined} label={editorT("count.maximum", "Maximum files")}>
                <TextInput disabled={readOnly} min={0} type="number" value={schema.maxCount} onChange={(event) => patch({ maxCount: Number(event.target.value) })} />
              </Field>
            </div>
          </section>

          <section className="grid gap-5 border-t border-[var(--color-outline-variant)] pt-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("size.title", "Single file size")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("size.description", "Set the maximum size for each uploaded file.")}</p>
            </div>
            <Field error={errors.maxSizeBytes ? editorT("errors.maxSizeBytes", errors.maxSizeBytes) : undefined} label={editorT("size.maximum", "Maximum file size (bytes)")}>
              <TextInput disabled={readOnly} min={1} type="number" value={schema.maxSizeBytes} onChange={(event) => patch({ maxSizeBytes: Number(event.target.value) })} />
            </Field>
          </section>

          <section className="grid gap-5 border-t border-[var(--color-outline-variant)] pt-5 md:grid-cols-[220px_minmax(0,1fr)]">
            <div>
              <h3 className="text-sm font-semibold">{editorT("name.title", "File name rules")}</h3>
              <p className="mt-1 text-[12px] leading-5 text-[var(--color-on-surface-variant)]">{editorT("name.description", "Set file name rules to distinguish files in the same resource.")}</p>
            </div>
            <div className="grid gap-4">
              <Field error={errors.maxNameLength ? editorT("errors.maxNameLength", errors.maxNameLength) : undefined} label={editorT("name.maxLength", "Maximum file name length")}>
                <TextInput disabled={readOnly} min={1} type="number" value={schema.maxNameLength} onChange={(event) => patch({ maxNameLength: Number(event.target.value) })} />
              </Field>
              <Field error={errors.namePattern ? editorT("errors.namePattern", errors.namePattern) : undefined} label={editorT("name.pattern", "File name rule")}>
                <TextInput className="font-mono" disabled={readOnly} value={schema.namePattern} onChange={(event) => patch({ namePattern: event.target.value })} />
              </Field>
              <div className="flex items-start gap-3 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3">
                <LockKeyhole aria-hidden="true" className="mt-0.5 shrink-0 text-[var(--color-on-surface-variant)]" size={16} />
                <div>
                  <p className="text-sm font-medium">{editorT("name.unique", "File names must be unique within a resource version")}</p>
                  <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{editorT("name.uniqueDescription", "This rule is always enforced.")}</p>
                </div>
              </div>
            </div>
          </section>
        </div>
      )}
    </SurfaceCard>
  );
}
