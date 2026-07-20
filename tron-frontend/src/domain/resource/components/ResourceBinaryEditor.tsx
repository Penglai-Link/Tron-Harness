import {
  useId,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Download,
  FileUp,
  Plus,
  RefreshCcw,
  Trash2,
} from "lucide-react";

import {
  Button,
  buttonVariants,
} from "@/components/ui/button";
import type {
  ResourceBinaryMapping,
  ResourceDefinitionBinarySchema,
} from "@/domain/resource/model";
import { TextInput } from "@/shared";
import { cn } from "@/shared/lib/utils";

interface ResourceBinaryEditorProps {
  binaries: ResourceBinaryMapping[];
  onDownload: (binary: ResourceBinaryMapping) => void | Promise<void>;
  onRemove: (binary: ResourceBinaryMapping) => void | Promise<void>;
  onReplace: (binary: ResourceBinaryMapping, file: File) => void | Promise<void>;
  onUpload: (binaryName: string, file: File) => void | Promise<void>;
  readOnly: boolean;
  schema: ResourceDefinitionBinarySchema;
  staged?: boolean;
}

export function ResourceBinaryEditor({
  binaries,
  onDownload,
  onRemove,
  onReplace,
  onUpload,
  readOnly,
  schema,
  staged = false,
}: ResourceBinaryEditorProps) {
  const { t } = useTranslation("pages");
  const uploadInputId = useId();
  const [binaryName, setBinaryName] = useState("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const binaryT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`resourceEditor.binaries.${key}`, { defaultValue, ...options })
  );
  const maxReached = binaries.length >= schema.maxCount;
  const validName = Boolean(
    binaryName
    && binaryName.length <= schema.maxNameLength
    && (() => {
      try {
        return new RegExp(schema.namePattern).test(binaryName);
      } catch {
        return false;
      }
    })(),
  );

  function chooseFile(file: File | null) {
    if (pendingAction) return;
    setSelectedFile(file);
    if (file && !binaryName) setBinaryName(file.name.slice(0, schema.maxNameLength));
  }

  async function runAction(action: string, callback: () => void | Promise<void>) {
    if (pendingAction) return false;
    setPendingAction(action);
    try {
      await callback();
      return true;
    } catch {
      // The parent owns user-facing request errors. Retain input on failure so
      // a retry does not require choosing the file again.
      return false;
    } finally {
      setPendingAction(null);
    }
  }

  function upload() {
    if (!selectedFile || !validName || maxReached) return;
    const file = selectedFile;
    const name = binaryName;
    void runAction("upload", () => onUpload(name, file)).then((succeeded) => {
      if (!succeeded) return;
      setSelectedFile(null);
      setBinaryName("");
    });
  }

  return (
    <section aria-labelledby="resource-binaries-heading">
      <div className="mb-3 flex min-w-0 items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <h2 className="text-sm font-semibold" id="resource-binaries-heading">{binaryT("title", "Files")}</h2>
          <span className="rounded-full bg-[var(--color-surface-container)] px-2 py-0.5 font-mono text-[10px] text-[var(--color-on-surface-variant)]">
            {binaries.length} / {schema.maxCount}
          </span>
        </div>
      </div>

      {!readOnly && !maxReached ? (
        <div className="mb-3 grid min-w-0 gap-2 rounded-lg border border-dashed border-[var(--color-outline)] bg-[var(--color-surface-container-lowest)] p-3">
          <label className={cn(buttonVariants({ size: "sm", variant: "outline" }), "min-w-0 cursor-pointer justify-start overflow-hidden")} htmlFor={uploadInputId}>
            <FileUp aria-hidden="true" className="shrink-0" size={15} />
            <span className="truncate">{selectedFile?.name ?? binaryT("chooseFile", "Choose file")}</span>
          </label>
          <input
            accept="*/*"
            className="sr-only"
            disabled={Boolean(pendingAction)}
            id={uploadInputId}
            type="file"
            onChange={(event) => chooseFile(event.target.files?.[0] ?? null)}
          />
          <div className="flex min-w-0 gap-2">
            <TextInput
              aria-label={binaryT("name", "Stored file name")}
              className="h-9 min-w-0 flex-1 font-mono text-[12px]"
              disabled={Boolean(pendingAction)}
              maxLength={schema.maxNameLength}
              placeholder={binaryT("namePlaceholder", "Stored file name")}
              value={binaryName}
              onChange={(event) => setBinaryName(event.target.value)}
            />
            <Button className="shrink-0" disabled={Boolean(pendingAction) || !selectedFile || !validName} size="sm" onClick={upload}>
              <Plus aria-hidden="true" size={14} />
              {staged ? binaryT("add", "Add") : binaryT("upload", "Upload")}
            </Button>
          </div>
          <p className="text-[12px] text-[var(--color-on-surface-variant)]">
            {binaryT("nameHint", "Uses the selected file name by default. Change it if needed.")}
          </p>
        </div>
      ) : null}

      <div className="overflow-hidden rounded-lg border border-[var(--color-outline-variant)]">
        {binaries.map((binary, index) => {
          const replaceId = `${uploadInputId}-replace-${index}`;
          const isPending = Boolean(pendingAction);
          return (
            <div aria-busy={isPending} className="flex min-h-12 min-w-0 items-center gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2 last:border-b-0" key={binary.binaryName}>
              <span className="min-w-0 flex-1 truncate font-mono text-[12px]" title={binary.binaryName}>{binary.binaryName}</span>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  aria-label={binaryT("downloadNamed", "Download {{name}}", { name: binary.binaryName })}
                  disabled={isPending}
                  size="icon-sm"
                  variant="ghost"
                  onClick={() => { void runAction(`download:${binary.binaryName}`, () => onDownload(binary)); }}
                >
                  <Download aria-hidden="true" size={14} />
                </Button>
                {!readOnly ? (
                  <>
                    <label
                      aria-label={binaryT("replaceNamed", "Replace {{name}}", { name: binary.binaryName })}
                      aria-disabled={isPending}
                      className={cn(buttonVariants({ size: "icon-sm", variant: "ghost" }), isPending ? "pointer-events-none opacity-50" : "cursor-pointer")}
                      htmlFor={replaceId}
                    >
                      <RefreshCcw aria-hidden="true" size={14} />
                    </label>
                    <input
                      accept="*/*"
                      className="sr-only"
                      disabled={isPending}
                      id={replaceId}
                      type="file"
                      onChange={(event) => {
                        const file = event.target.files?.[0];
                        if (file) void runAction(`replace:${binary.binaryName}`, () => onReplace(binary, file));
                        event.target.value = "";
                      }}
                    />
                    <Button
                      aria-label={binaryT("removeNamed", "Remove {{name}}", { name: binary.binaryName })}
                      disabled={isPending}
                      size="icon-sm"
                      variant="ghost"
                      onClick={() => { void runAction(`remove:${binary.binaryName}`, () => onRemove(binary)); }}
                    >
                      <Trash2 aria-hidden="true" size={14} />
                    </Button>
                  </>
                ) : null}
              </div>
            </div>
          );
        })}
        {!binaries.length ? (
          <p className="px-3 py-4 text-center text-[12px] text-[var(--color-on-surface-variant)]">
            {binaryT("empty", "No files added yet")}
          </p>
        ) : null}
      </div>
    </section>
  );
}
