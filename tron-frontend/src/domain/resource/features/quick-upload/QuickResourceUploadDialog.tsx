import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type DragEvent,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import {
  ChevronDown,
  CircleAlert,
  FileUp,
  LoaderCircle,
  Upload,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import type {
  ResourceKind,
  QuickResourceUploadHandoffPayload,
} from "@/domain/resource/model";
import {
  SelectInput,
  TextArea,
  TextInput,
  ToggleSwitch,
} from "@/shared";
import { cn } from "@/shared/lib/utils";

import { readPdfPageCount } from "./pdfPageCount";
import {
  evaluateQuickResourceUpload,
  generateQuickResourceCode,
  getFileStem,
  quickResourceTypes,
  suggestQuickResourceType,
  type QuickResourceUploadDecision,
  type QuickResourceType,
} from "./quickResourceUpload";

export interface QuickResourceCreateCommand {
  allowInheritance: boolean;
  description: string;
  file: File;
  kind: "FILE" | "IMAGE" | "PDF";
  name: string;
  resourceCode: string;
  workspaceCode: string;
}

export interface QuickResourceCreatedResult {
  kind: ResourceKind;
  resourceCode: string;
}

interface QuickResourceUploadDialogProps {
  open: boolean;
  workspaceCode: string;
  onClose: () => void;
  onCreated: (result: QuickResourceCreatedResult) => void;
  onContinueInEditor?: (payload: QuickResourceUploadHandoffPayload) => void;
  onCreate: (command: QuickResourceCreateCommand) => Promise<QuickResourceCreatedResult>;
}

type PdfReadState =
  | { status: "IDLE" }
  | { status: "LOADING" }
  | { pageCount: number; status: "READY" }
  | { status: "FAILED" };

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "";
}

function formatFileSize(size: number, locale: string) {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(size / 1024)} KB`;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(size / (1024 * 1024))} MB`;
}

export function QuickResourceUploadDialog({
  open,
  workspaceCode,
  onClose,
  onCreated,
  onContinueInEditor,
  onCreate,
}: QuickResourceUploadDialogProps) {
  const { i18n, t } = useTranslation("pages");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const hasSelectedFileRef = useRef(false);
  const editorHandoffStartedRef = useRef(false);
  const onCloseRef = useRef(onClose);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const submittingRef = useRef(false);
  const [allowInheritance, setAllowInheritance] = useState(false);
  const [description, setDescription] = useState("");
  const [error, setError] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [moreOpen, setMoreOpen] = useState(false);
  const [name, setName] = useState("");
  const [pdfReadState, setPdfReadState] = useState<PdfReadState>({ status: "IDLE" });
  const [resourceCode, setResourceCode] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [type, setType] = useState<QuickResourceType>("FILE");

  const quickT = (key: string, options?: Record<string, unknown>) => (
    t(`resources.quickUpload.${key}`, options)
  );

  const pageCount = pdfReadState.status === "READY" ? pdfReadState.pageCount : type === "PDF" ? 0 : null;
  const decision = useMemo(() => {
    if (!file) return null;
    return evaluateQuickResourceUpload({
      binaryName: file.name,
      file,
      pageCount,
      type,
    });
  }, [file, pageCount, type]);
  const blockedMessage = decision?.kind === "BLOCKED"
    ? quickT(`blocked.${decision.reason.toLowerCase()}`)
    : "";
  const requiresEditor = decision?.kind === "EDITOR";
  const editorRequiredWithoutHandoff = requiresEditor && !onContinueInEditor;
  // When a PDF is first selected its state is still IDLE until the reader
  // effect starts. Treat that first render as pending too; otherwise the
  // automatic editor handoff could win the race with a successful read.
  const waitingForPdf = type === "PDF" && (
    pdfReadState.status === "IDLE" || pdfReadState.status === "LOADING"
  );
  hasSelectedFileRef.current = Boolean(file);
  onCloseRef.current = onClose;
  submittingRef.current = submitting;

  function reset() {
    editorHandoffStartedRef.current = false;
    setAllowInheritance(false);
    setDescription("");
    setError("");
    setFile(null);
    setMoreOpen(false);
    setName("");
    setPdfReadState({ status: "IDLE" });
    setResourceCode("");
    setSubmitting(false);
    setType("FILE");
  }

  useEffect(() => {
    if (!open) return;
    previousFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    reset();
    const focusTimer = window.setTimeout(() => fileInputRef.current?.focus(), 0);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || submittingRef.current || hasSelectedFileRef.current) return;
      onCloseRef.current();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      window.removeEventListener("keydown", handleKeyDown);
      previousFocusRef.current?.focus();
    };
  // Reset only when the dialog opens. File/type changes must keep the user's
  // in-progress selection intact while the PDF preflight runs.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!file || type !== "PDF") {
      setPdfReadState({ status: "IDLE" });
      return;
    }
    let cancelled = false;
    setPdfReadState({ status: "LOADING" });
    void readPdfPageCount(file)
      .then((nextPageCount) => {
        if (cancelled || nextPageCount < 1) return;
        setPdfReadState({ pageCount: nextPageCount, status: "READY" });
      })
      .catch(() => {
        if (!cancelled) setPdfReadState({ status: "FAILED" });
      });
    return () => {
      cancelled = true;
    };
  }, [file, type]);

  function continueInEditor(nextDecision: Extract<QuickResourceUploadDecision, { kind: "EDITOR" }>) {
    if (!file || !onContinueInEditor || editorHandoffStartedRef.current) return;
    editorHandoffStartedRef.current = true;
    onContinueInEditor({
      allowInheritance,
      binaryName: nextDecision.binaryName,
      description,
      file,
      kind: nextDecision.type,
      name: name.trim(),
      pageCount,
      resourceCode: resourceCode.trim(),
      workspaceCode,
    });
  }

  useEffect(() => {
    if (!open || !decision || decision.kind !== "EDITOR" || waitingForPdf || !onContinueInEditor) return;
    continueInEditor(decision);
  // A repairable preflight result is intentionally terminal for quick upload:
  // the full editor is the only surface where the missing information can be
  // corrected. The ref makes the transition stable across effect replays.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [decision, open, waitingForPdf]);

  function close() {
    if (submitting) return;
    onClose();
  }

  function selectFiles(files: FileList | File[]) {
    const selected = Array.from(files);
    if (selected.length !== 1) {
      setError(quickT("errors.singleFile"));
      return;
    }
    const selectedFile = selected[0];
    const suggestedType = suggestQuickResourceType(selectedFile.name);
    const generatedAt = Date.now();
    const generatedCode = generateQuickResourceCode(selectedFile.name, generatedAt);
    setError("");
    setFile(selectedFile);
    setType(suggestedType);
    setName(getFileStem(selectedFile.name) || selectedFile.name);
    setResourceCode(generatedCode);
    // A fallback code needs an explicit user-visible affordance so it is not
    // mistaken for a silently generated collision suffix.
    setMoreOpen(generatedCode === `FILE_${generatedAt}`);
    setDescription("");
    setAllowInheritance(false);
  }

  function handleFileInput(event: ChangeEvent<HTMLInputElement>) {
    if (event.target.files) selectFiles(event.target.files);
    event.target.value = "";
  }

  function handleDrop(event: DragEvent<HTMLLabelElement>) {
    event.preventDefault();
    if (submitting || !event.dataTransfer.files.length) return;
    selectFiles(event.dataTransfer.files);
  }

  async function submit() {
    if (!file || !decision || submitting || waitingForPdf) return;
    if (decision.kind === "BLOCKED") {
      setError(blockedMessage);
      return;
    }
    if (decision.kind === "EDITOR") {
      if (onContinueInEditor) continueInEditor(decision);
      return;
    }

    setSubmitting(true);
    setError("");
    try {
      const created = await onCreate({
        allowInheritance,
        description,
        file,
        kind: type,
        name: name.trim(),
        resourceCode: resourceCode.trim(),
        workspaceCode,
      });
      onCreated(created);
    } catch (cause) {
      setMoreOpen(true);
      setError(errorMessage(cause) || quickT("errors.create"));
    } finally {
      setSubmitting(false);
    }
  }

  if (!open || typeof document === "undefined") return null;

  return createPortal(
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm">
      <section
        aria-label={quickT("aria")}
        aria-modal="true"
        className="flex max-h-[min(46rem,calc(100vh-2rem))] w-[min(94vw,38rem)] flex-col overflow-hidden rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
        role="dialog"
      >
        <header className="flex items-start gap-4 border-b border-[var(--color-outline-variant)] px-5 py-4">
          <div className="min-w-0 flex-1">
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{quickT("title")}</h2>
          </div>
          <Button aria-label={quickT("actions.close")} disabled={submitting} size="icon-sm" type="button" variant="ghost" onClick={close}>
            <X aria-hidden="true" size={16} />
          </Button>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          <input
            accept="*/*"
            aria-label={quickT("fileInputAria")}
            className="sr-only"
            disabled={submitting}
            id="quick-resource-upload-input"
            ref={fileInputRef}
            type="file"
            onChange={handleFileInput}
          />
          <label
            className={cn(
              "flex min-h-32 cursor-pointer flex-col items-center justify-center rounded-xl border border-dashed border-[var(--color-outline)] bg-[var(--color-surface-container-low)] px-5 py-4 text-center transition-colors hover:border-[var(--color-primary)] hover:bg-[var(--color-primary-container)]/35",
              submitting && "pointer-events-none opacity-60",
            )}
            data-testid="quick-resource-upload-dropzone"
            htmlFor="quick-resource-upload-input"
            onDragOver={(event) => event.preventDefault()}
            onDrop={handleDrop}
          >
            <FileUp aria-hidden="true" className="mb-2 text-[var(--color-on-surface-variant)]" size={22} />
            <span className="text-sm font-medium text-[var(--color-on-surface)]">{file ? file.name : quickT("chooseFile")}</span>
            <span className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">{quickT("dropHint")}</span>
          </label>

          {file ? (
            <div className="mt-4 space-y-4">
              <div className="flex min-w-0 items-center justify-between gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-2.5">
                <span className="min-w-0 truncate text-sm font-medium">{file.name}</span>
                <span className="shrink-0 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{formatFileSize(file.size, i18n.language)}</span>
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                <label className="grid gap-1.5 text-sm font-medium text-[var(--color-on-surface)]">
                  {quickT("fields.name")}
                  <TextInput
                    aria-label={quickT("fields.name")}
                    disabled={submitting}
                    maxLength={255}
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                  />
                </label>
                <label className="grid gap-1.5 text-sm font-medium text-[var(--color-on-surface)]">
                  {quickT("fields.definition")}
                  <SelectInput
                    aria-label={quickT("fields.definition")}
                    disabled={submitting}
                    popupPositionerClassName="z-[120]"
                    value={type}
                    onValueChange={(value) => {
                      setError("");
                      setType(value as QuickResourceType);
                    }}
                  >
                    {quickResourceTypes.map((item) => <SelectItem key={item} value={item}>{quickT(`type.${item.toLowerCase()}`)}</SelectItem>)}
                  </SelectInput>
                </label>
              </div>

              {type === "PDF" ? (
                <p className={cn("text-[12px]", pdfReadState.status === "FAILED" ? "text-[var(--color-error)]" : "text-[var(--color-on-surface-variant)]")} role={pdfReadState.status === "FAILED" ? "status" : undefined}>
                  {pdfReadState.status === "LOADING" ? quickT("pdf.reading") : null}
                  {pdfReadState.status === "READY" ? quickT("pdf.pages", { count: pdfReadState.pageCount }) : null}
                  {pdfReadState.status === "FAILED" ? quickT("pdf.failed") : null}
                </p>
              ) : null}

              <details open={moreOpen} onToggle={(event) => setMoreOpen(event.currentTarget.open)}>
                <summary className="flex cursor-pointer list-none items-center gap-1 text-sm font-medium text-[var(--color-on-surface-variant)] marker:hidden">
                  <ChevronDown aria-hidden="true" className="transition-transform [[open]_&]:rotate-180" size={15} />
                  {quickT("more.title")}
                </summary>
                <div className="mt-3 grid gap-3 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
                  <label className="grid gap-1.5 text-sm font-medium text-[var(--color-on-surface)]">
                    {quickT("fields.code")}
                    <TextInput
                      aria-label={quickT("fields.code")}
                      disabled={submitting}
                      maxLength={64}
                      value={resourceCode}
                      onChange={(event) => setResourceCode(event.target.value)}
                    />
                  </label>
                  <label className="grid gap-1.5 text-sm font-medium text-[var(--color-on-surface)]">
                    {quickT("fields.description")}
                    <TextArea
                      aria-label={quickT("fields.description")}
                      className="min-h-20 bg-background"
                      disabled={submitting}
                      placeholder={quickT("fields.descriptionPlaceholder")}
                      value={description}
                      onChange={(event) => setDescription(event.target.value)}
                    />
                  </label>
                  <ToggleSwitch
                    ariaLabel={quickT("fields.inheritance")}
                    checked={allowInheritance}
                    disabled={submitting}
                    label={quickT("fields.inheritance")}
                    onCheckedChange={setAllowInheritance}
                  />
                </div>
              </details>

              {blockedMessage ? (
                <div className="flex items-start gap-[12px] rounded-lg border border-[var(--color-error)]/35 bg-[var(--color-error-container)] px-3 py-2 text-[12px] text-[var(--color-on-error-container)]" role="alert">
                  <CircleAlert aria-hidden="true" className="mt-0.5 shrink-0" size={15} />
                  <span>{blockedMessage}</span>
                </div>
              ) : null}
              {requiresEditor ? (
                <p
                  className={cn(
                    "text-[12px]",
                    editorRequiredWithoutHandoff ? "text-[var(--color-error)]" : "text-[var(--color-on-surface-variant)]",
                  )}
                  role={editorRequiredWithoutHandoff ? "alert" : undefined}
                >
                  {editorRequiredWithoutHandoff ? quickT("errors.editorRequired") : quickT("editorRequired")}
                </p>
              ) : null}
            </div>
          ) : null}
          {error ? (
            <div className="mt-4 flex items-start gap-[12px] rounded-lg border border-[var(--color-error)]/35 bg-[var(--color-error-container)] px-3 py-2 text-[12px] text-[var(--color-on-error-container)]" role="alert">
              <CircleAlert aria-hidden="true" className="mt-0.5 shrink-0" size={15} />
              <span>{error}</span>
            </div>
          ) : null}
        </div>

        <footer className="flex justify-end gap-[12px] border-t border-[var(--color-outline-variant)] px-5 py-4">
          <Button disabled={submitting} type="button" variant="ghost" onClick={close}>{quickT("actions.cancel")}</Button>
          <Button
            aria-busy={submitting}
            disabled={!file || submitting || waitingForPdf || decision?.kind === "BLOCKED" || editorRequiredWithoutHandoff || !name.trim() || !resourceCode.trim()}
            type="button"
            onClick={() => { void submit(); }}
          >
            {submitting ? <LoaderCircle aria-hidden="true" className="animate-spin" size={15} /> : requiresEditor && onContinueInEditor ? null : <Upload aria-hidden="true" size={15} />}
            {submitting ? quickT("actions.creating") : requiresEditor && onContinueInEditor ? quickT("actions.continue") : quickT("actions.create")}
          </Button>
        </footer>
      </section>
    </div>,
    document.body,
  );
}
