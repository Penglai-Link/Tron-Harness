import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  CheckCircle2,
  Code2,
  FlaskConical,
  Maximize2,
  Minimize2,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Field, TextArea } from "@/components/forms";
import { SegmentedTabs } from "@/components/inspector";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/shared/lib/utils";

import {
  createJsonSample,
  parseRootObjectSchema,
  serializeJsonSchema,
  validateJsonValue,
} from "./schema";
import type { JsonSchemaDraft7EditorProps } from "./types";
import { JsonSchemaDraft7VisualEditor } from "./JsonSchemaDraft7VisualEditor";

const defaultFormatOptions = [
  { value: "date" },
  { value: "date-time" },
  { value: "time" },
  { value: "email" },
  { value: "uri" },
  { value: "uuid" },
  { value: "hostname" },
  { value: "ipv4" },
  { value: "ipv6" },
];

/**
 * Tron-controlled Draft 7 editor copied and adapted from JSONJoy Builder.
 *
 * The field-tree and recursive type-editor structure derives from
 * lovasoa/jsonjoy-builder (MIT, commit 4c86297). Tron replaces the upstream
 * registry, primitives, styling, localization, JSON editor, and state boundary.
 * See NOTICE.md. This component has no runtime dependency on that project.
 */
export function JsonSchemaDraft7Editor({
  formatOptions = [],
  jsonFileName = "schema.json",
  onChange,
  onValidityChange,
  readOnly = false,
  sampleData,
  schema,
}: JsonSchemaDraft7EditorProps) {
  const { t } = useTranslation("common");
  const tr = (key: string, fallback: string, options?: Record<string, unknown>) => t(`jsonSchemaEditor.${key}`, { defaultValue: fallback, ...options });
  const [jsonText, setJsonText] = useState(() => serializeJsonSchema(schema));
  const [jsonError, setJsonError] = useState("");
  const [mobilePane, setMobilePane] = useState("visual");
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [visualWidth, setVisualWidth] = useState(55);
  const [testOpen, setTestOpen] = useState(false);
  const [testData, setTestData] = useState(() => sampleData ?? JSON.stringify(createJsonSample(schema), null, 2));
  const [testResult, setTestResult] = useState<{ tone: "error" | "success"; text: string } | null>(null);
  const splitRef = useRef<HTMLDivElement | null>(null);
  const serializedSchema = useMemo(() => serializeJsonSchema(schema), [schema]);
  const effectiveFormats = useMemo(() => {
    const options = [...defaultFormatOptions, ...formatOptions];
    return Array.from(new Map(options.map((option) => [option.value, option])).values());
  }, [formatOptions]);

  useEffect(() => {
    setJsonText(serializedSchema);
    setJsonError("");
  }, [serializedSchema]);

  useEffect(() => {
    onValidityChange?.(!jsonError);
  }, [jsonError, onValidityChange]);

  function updateJson(value: string) {
    setJsonText(value);
    if (readOnly) return;
    try {
      const parsed = parseRootObjectSchema(value);
      setJsonError("");
      onChange(parsed);
    } catch (error) {
      setJsonError(error instanceof Error ? error.message : tr("errors.invalidJson", "JSON is invalid."));
    }
  }

  function startResize(event: React.PointerEvent<HTMLButtonElement>) {
    if (!splitRef.current) return;
    event.preventDefault();
    const container = splitRef.current;

    function move(pointerEvent: PointerEvent) {
      const rect = container.getBoundingClientRect();
      const next = ((pointerEvent.clientX - rect.left) / rect.width) * 100;
      setVisualWidth(Math.min(68, Math.max(36, next)));
    }

    function stop() {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    }

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  }

  function validateTestData() {
    try {
      const data = JSON.parse(testData) as unknown;
      const errors = validateJsonValue(schema, data);
      if (errors.length) throw new Error(errors.slice(0, 6).join("\n"));
      setTestResult({ tone: "success", text: tr("test.valid", "Test data matches this Schema.") });
    } catch (error) {
      setTestResult({
        tone: "error",
        text: error instanceof Error ? error.message : tr("test.invalid", "Test data is invalid."),
      });
    }
  }

  const visualPane = (
    <JsonSchemaDraft7VisualEditor
      formatOptions={effectiveFormats}
      readOnly={readOnly}
      schema={schema}
      onChange={onChange}
    />
  );

  const jsonPane = (
    <div className="flex min-h-[540px] min-w-0 flex-col bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)]">
      <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-2.5">
        <span className="inline-flex items-center gap-[12px] text-[12px] font-medium text-[var(--color-on-surface-variant)]">
          <Code2 aria-hidden="true" size={14} />
          {jsonFileName}
        </span>
        <Badge variant={jsonError ? "error" : "success"}>{jsonError ? tr("json.invalid", "INVALID JSON") : tr("json.valid", "VALID JSON")}</Badge>
      </div>
      <TextArea
        aria-label={tr("json.aria", "JSON Schema source")}
        className="min-h-0 flex-1 resize-none rounded-none border-0 bg-transparent p-4 font-mono text-[12px] leading-6 text-[var(--color-on-surface)] shadow-none selection:bg-[var(--color-accent-container)] focus-visible:ring-0"
        readOnly={readOnly}
        spellCheck={false}
        value={jsonText}
        wrap="off"
        onChange={(event) => updateJson(event.target.value)}
      />
      <div className={cn(
        "min-h-10 whitespace-pre-line border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-2 text-[11px]",
        jsonError ? "bg-red-50 text-red-700" : "text-[var(--color-on-surface-variant)]",
      )}
      >
        {jsonError || tr("json.help", "Draft 7 keywords without visual controls are preserved here.")}
      </div>
    </div>
  );

  return (
    <section className={cn(
      "overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]",
      isFullscreen ? "fixed inset-3 z-[90] flex flex-col shadow-2xl" : null,
    )}
    >
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-4 py-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-[12px]">
            <h2 className="text-base font-semibold">{tr("title", "Information fields")}</h2>
            <Badge variant="success">Draft 7</Badge>
            {readOnly ? <Badge variant="outline">{tr("readOnly", "READ ONLY")}</Badge> : null}
          </div>
          <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{tr("subtitle", "Visual fields and JSON edit the same field settings.")}</p>
        </div>
        <div className="flex items-center gap-[12px]">
          <Button size="sm" variant="outline" onClick={() => { setTestResult(null); setTestOpen(true); }}>
            <FlaskConical aria-hidden="true" size={14} />
            {tr("actions.test", "Test entry")}
          </Button>
          <Button aria-label={isFullscreen ? tr("actions.exitFullscreen", "Exit fullscreen") : tr("actions.fullscreen", "Open fullscreen")} size="icon-sm" variant="ghost" onClick={() => setIsFullscreen((current) => !current)}>
            {isFullscreen ? <Minimize2 aria-hidden="true" size={15} /> : <Maximize2 aria-hidden="true" size={15} />}
          </Button>
        </div>
      </header>

      <div className="border-b border-[var(--color-outline-variant)] px-4 py-2 lg:hidden">
        <SegmentedTabs
          items={[
            { label: tr("tabs.visual", "Visual"), value: "visual" },
            { label: tr("tabs.json", "JSON"), value: "json" },
          ]}
          value={mobilePane}
          onValueChange={setMobilePane}
        />
      </div>

      <div ref={splitRef} className="min-h-0 flex-1 lg:grid" style={{ gridTemplateColumns: `${visualWidth}% 8px minmax(0, 1fr)` }}>
        <div className={cn("min-h-0 overflow-y-auto", mobilePane === "visual" ? "block" : "hidden", "lg:block")}>{visualPane}</div>
        <button
          aria-label={tr("actions.resize", "Resize visual and JSON panes")}
          className="group hidden cursor-col-resize items-center justify-center border-x border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] hover:bg-[var(--color-hover-surface)] lg:flex"
          type="button"
          onPointerDown={startResize}
        >
          <span className="h-10 w-0.5 rounded-full bg-[var(--color-outline)] transition-colors group-hover:bg-[var(--color-on-surface-variant)]" />
        </button>
        <div className={cn("min-h-0", mobilePane === "json" ? "block" : "hidden", "lg:block")}>{jsonPane}</div>
      </div>

      {testOpen ? (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/45 p-4" role="presentation" onMouseDown={(event) => { if (event.currentTarget === event.target) setTestOpen(false); }}>
          <section aria-labelledby="json-schema-test-title" aria-modal="true" className="w-full max-w-2xl overflow-hidden rounded-2xl bg-[var(--color-elevated-surface)] shadow-2xl" role="dialog">
            <header className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
              <h2 id="json-schema-test-title" className="text-lg font-semibold">{tr("test.title", "Test entry")}</h2>
                <p className="mt-0.5 text-[12px] text-[var(--color-on-surface-variant)]">{tr("test.description", "Check an entry against the current field rules.")}</p>
              </div>
              <Button aria-label={tr("test.close", "Close test data dialog")} size="icon-sm" variant="ghost" onClick={() => setTestOpen(false)}><X aria-hidden="true" size={15} /></Button>
            </header>
            <div className="space-y-4 p-5">
              <Field label={tr("test.data", "Entry data (JSON)")}>
                <TextArea className="min-h-64 font-mono text-[12px] leading-6" spellCheck={false} value={testData} onChange={(event) => { setTestData(event.target.value); setTestResult(null); }} />
              </Field>
              {testResult ? (
                <div className={cn("flex items-start gap-[12px] whitespace-pre-line rounded-xl px-3 py-2.5 text-sm", testResult.tone === "success" ? "bg-emerald-50 text-emerald-800" : "bg-red-50 text-red-700")}>
                  {testResult.tone === "success" ? <CheckCircle2 aria-hidden="true" className="mt-0.5 shrink-0" size={16} /> : <AlertCircle aria-hidden="true" className="mt-0.5 shrink-0" size={16} />}
                  {testResult.text}
                </div>
              ) : null}
            </div>
            <footer className="flex justify-end gap-[12px] border-t border-[var(--color-outline-variant)] px-5 py-4">
              <Button variant="outline" onClick={() => setTestOpen(false)}>{tr("test.cancel", "Cancel")}</Button>
              <Button onClick={validateTestData}>{tr("test.validate", "Validate")}</Button>
            </footer>
          </section>
        </div>
      ) : null}
    </section>
  );
}
