import { useTranslation } from "react-i18next";
import { MarkdownContent } from "@/components/markdown-content";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

// Contributor IDs are extensible; the familiar aliases keep their richer previews.
export type ContextDetailBlock = "systemPrompt" | "toolSchema" | "applicationContext" | "history" | (string & {});
type RecordValue = Record<string, unknown>;
const keyPrefix = "chat.shortTermMemory.contextPanel.";

export function contextDetailRawValue(block: ContextDetailBlock, value: unknown): unknown {
  return record(value) ? value[block === "toolSchema" ? "tools" : "messages"] ?? [] : value;
}

function record(value: unknown): value is RecordValue {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function records(value: unknown): RecordValue[] {
  return Array.isArray(value) ? value.filter(record) : [];
}

function parse(value: unknown): unknown {
  if (typeof value !== "string") return value;
  try { return JSON.parse(value); } catch { return value; }
}

function taggedContent(text: string, tag: string): string | null {
  const match = text.trim().match(new RegExp(`^<${tag}>([\\s\\S]*?)</${tag}>$`));
  return match?.[1]?.trim() ?? null;
}

function Empty({ label }: { label: string }) {
  return <p className="py-2 text-xs text-[var(--color-on-surface-variant)]">{label}</p>;
}

/** Structured product content stays flat; protocol/report envelopes use the raw view. */
function StructuredContent({ value }: { value: unknown }) {
  const { t } = useTranslation("pages");
  if (value == null) return null;
  if (typeof value === "string") return <MarkdownContent content={value} className="text-xs leading-6 [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-xs" />;
  if (Array.isArray(value)) return <div className="space-y-3">{value.map((item, index) => <StructuredContent key={index} value={item} />)}</div>;
  if (!record(value)) return <span>{typeof value === "boolean" ? t(`chat.applications.preview.${value ? "yes" : "no"}`) : String(value)}</span>;
  return <dl className="space-y-2">{Object.entries(value).filter(([, item]) => item != null).map(([key, item]) => (
    <div className="min-w-0" key={key}>
      <dt className="text-xs font-medium text-[var(--color-on-surface-variant)]">{t(`chat.applications.preview.fields.${key}`, { defaultValue: key })}</dt>
      <dd className="min-w-0 whitespace-pre-wrap break-words text-xs leading-6">{key === "status" && typeof item === "string" && ["Ready", "NotGit", "MissingDirectory", "Unavailable"].includes(item)
        ? t(item === "Ready" ? "chat.applications.preview.ready" : `chat.applications.status.${item}`)
        : <StructuredContent value={item} />}</dd>
    </div>
  ))}</dl>;
}

function PromptContent({ text }: { text: string }) {
  // JSON catalog data may be embedded directly in prompt prose. Preserve the
  // prose and show its structured value without adding another framed surface.
  for (let start = 0; start < text.length; start += 1) {
    if (text[start] !== "{" && text[start] !== "[") continue;
    const stack: string[] = [];
    let inString = false;
    let escaped = false;
    for (let end = start; end < text.length; end += 1) {
      const character = text[end];
      if (inString) {
        if (escaped) escaped = false;
        else if (character === "\\") escaped = true;
        else if (character === '"') inString = false;
        continue;
      }
      if (character === '"') inString = true;
      else if (character === "{" || character === "[") stack.push(character === "{" ? "}" : "]");
      else if (character === "}" || character === "]") {
        if (stack.pop() !== character) break;
        if (stack.length) continue;
        const value = parse(text.slice(start, end + 1));
        if (typeof value === "string") break;
        return <div className="space-y-3">
          {text.slice(0, start).trim() ? <MarkdownContent content={text.slice(0, start)} className="text-xs leading-6" /> : null}
          <StructuredContent value={value} />
          {text.slice(end + 1).trim() ? <PromptContent text={text.slice(end + 1)} /> : null}
        </div>;
      }
    }
  }
  return <MarkdownContent content={text} className="text-xs leading-6 [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-xs" />;
}

function MessagePart({ part }: { part: RecordValue }) {
  const { t } = useTranslation("pages");
  if (part.type === "text") return <PromptContent text={String(part.text ?? "")} />;
  if (part.type === "context") {
    const value = parse(part.text);
    if (record(value) && record(value.answer) && record(value.answer.answers)) {
      return <div className="space-y-1">
        <p className="text-xs font-medium text-[var(--color-on-surface-variant)]">{t(`${keyPrefix}historyAnswer`)}</p>
        {Object.values(value.answer.answers).map((answer, index) => <StructuredContent key={index} value={record(answer) ? answer.text ?? answer : answer} />)}
      </div>;
    }
    return <StructuredContent value={value} />;
  }
  if (part.type === "toolCall" || part.type === "toolResult") return <div className="space-y-2">
    <p className="text-xs font-medium">{t(`${keyPrefix}${part.type === "toolCall" ? "historyToolCall" : "historyToolResult"}`)} · <code>{String(part.tool_name ?? "")}</code></p>
    <StructuredContent value={parse(part.type === "toolCall" ? part.input : part.output)} />
  </div>;
  if (part.type === "compactionCheckpoint") return <StructuredContent value={part.summary} />;
  if (part.type === "image") return <p>{t(`${keyPrefix}historyImage`)}</p>;
  if (part.type === "reasoningSummary") return <StructuredContent value={part.text} />;
  return null;
}

function MessagesPreview({ value, history = false, systemPrompt = false, onOpenPrompt }: { value: unknown; history?: boolean; systemPrompt?: boolean; onOpenPrompt?: (code: string) => void }) {
  const { t } = useTranslation("pages");
  const messages = record(value) ? records(value.messages) : [];
  if (!messages.length) return <Empty label={t(`${keyPrefix}${history ? "historyEmpty" : systemPrompt ? "detailPromptEmpty" : "contributorEmpty"}`)} />;
  return <div className="divide-y divide-[var(--color-outline-variant)]">
    {history ? <p className="pb-3 text-xs text-[var(--color-on-surface-variant)]">{t(`${keyPrefix}historyNote`)}</p> : null}
    {messages.map((message, index) => <section className="space-y-3 py-4 first:pt-0 last:pb-0" key={index}>
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-xs font-semibold">{t(`${keyPrefix}detailPromptMessage`, { index: index + 1 })} · {t(`${keyPrefix}roles.${String(message.role)}`, { defaultValue: String(message.role) })}</h3>
        {typeof message.promptCode === "string" && onOpenPrompt ? <button type="button" className="text-xs text-[var(--color-primary)] hover:underline" onClick={() => onOpenPrompt(message.promptCode as string)}>{t(`${keyPrefix}detailPromptManagement`)}</button> : null}
      </div>
      <div className="space-y-3 text-xs leading-6">{records(message.parts).map((part, partIndex) => <MessagePart key={partIndex} part={part} />)}</div>
    </section>)}
  </div>;
}

function ToolPreview({ value }: { value: unknown }) {
  const { t } = useTranslation("pages");
  const tools = record(value) ? records(value.tools) : [];
  if (!tools.length) return <Empty label={t(`${keyPrefix}toolsEmpty`)} />;
  return <div className="divide-y divide-[var(--color-outline-variant)]">{tools.map((tool, index) => {
    const schema = record(tool.inputSchema) ? tool.inputSchema : {};
    const properties = record(schema.properties) ? Object.entries(schema.properties) : [];
    const required = Array.isArray(schema.required) ? schema.required : [];
    return <section className="space-y-3 py-4 first:pt-0 last:pb-0" key={index}>
      <div className="flex flex-wrap items-baseline justify-between gap-2"><h3 className="break-all font-mono text-xs font-semibold">{String(tool.toolName ?? "")}</h3><span className="text-xs text-[var(--color-on-surface-variant)]">{String(tool.domainCode ?? "")}</span></div>
      {typeof tool.description === "string" ? <p className="text-xs leading-6">{tool.description}</p> : null}
      {properties.length ? <Table>
        <TableHeader><TableRow><TableHead>{t(`${keyPrefix}toolParameter`)}</TableHead><TableHead>{t(`${keyPrefix}toolParameterType`)}</TableHead><TableHead>{t(`${keyPrefix}toolParameterDescription`)}</TableHead></TableRow></TableHeader>
        <TableBody>{properties.map(([name, property]) => <TableRow key={name}>
          <TableCell className="align-top font-mono text-xs">{name}{required.includes(name) ? <span aria-label={t(`${keyPrefix}toolParameterRequired`)}> *</span> : null}</TableCell>
          <TableCell className="align-top text-xs">{record(property) ? String(property.type ?? "—") : "—"}</TableCell>
          <TableCell className="whitespace-normal text-xs leading-5">{record(property) && typeof property.description === "string" ? property.description : "—"}</TableCell>
        </TableRow>)}</TableBody>
      </Table> : <Empty label={t(`${keyPrefix}toolNoParameters`)} />}
      <details className="text-xs"><summary className="cursor-pointer text-[var(--color-on-surface-variant)]">{t(`${keyPrefix}toolInputSchema`)}</summary><pre className="mt-2 whitespace-pre-wrap break-words font-mono text-xs leading-5">{JSON.stringify(schema, null, 2)}</pre></details>
    </section>;
  })}</div>;
}

function ApplicationScopePreview({ scope }: { scope: RecordValue }) {
  const { t } = useTranslation("pages");
  const apps = records(scope.applications);
  if (apps.length) return <div className="divide-y divide-[var(--color-outline-variant)]">{apps.map((app, index) => <section className="space-y-3 py-4 first:pt-0 last:pb-0" key={index}>
    <h3 className="text-xs font-semibold">{String(app.appName ?? app.appCode ?? "")}</h3>
    <StructuredContent value={{
      appCode: app.appCode,
      applicationKind: app.applicationKind,
      branch: app.resolvedBranch ?? app.selectedBranch ?? (typeof app.headCommit === "string" ? t("chat.applications.detached", { commit: app.headCommit.slice(0, 8) }) : t("chat.applications.unknown")),
      workingDirectory: app.workingDirectory ?? app.configuredDirectory,
      configuredDirectory: app.configuredDirectory !== app.workingDirectory ? app.configuredDirectory : null,
      worktreePath: app.worktreePath !== app.workingDirectory ? app.worktreePath : null,
      status: app.status !== "Ready" ? app.status : null,
    }} />
  </section>)}</div>;
  if (scope?.localDirectory) return <StructuredContent value={{ workingDirectory: scope.localDirectory }} />;
  return <Empty label={t("chat.applications.noSnapshot")} />;
}

function ToolDomainsPreview({ domains }: { domains: RecordValue[] }) {
  const { t } = useTranslation("pages");
  return <section className="space-y-2">
    <div className="flex items-baseline justify-between gap-3">
      <h3 className="text-xs font-semibold">{t("chat.applications.preview.toolDomains")}</h3>
      <span className="shrink-0 text-[11px] text-[var(--color-on-surface-variant)]">{t("chat.applications.preview.domainCount", { count: domains.length })}</span>
    </div>
    <div className="divide-y divide-[var(--color-outline-variant)] border-y border-[var(--color-outline-variant)]">
      {domains.map((domain, index) => {
        const code = typeof domain.domainCode === "string" ? domain.domainCode : "";
        const name = typeof domain.name === "string" ? domain.name : code;
        const sampleTools = Array.isArray(domain.sampleTools) ? domain.sampleTools.filter((tool): tool is string => typeof tool === "string") : [];
        const toolCount = typeof domain.toolCount === "number" ? domain.toolCount : sampleTools.length;
        return <details className="group py-2" key={`${code}:${index}`}>
          <summary className="cursor-pointer text-xs marker:text-[var(--color-on-surface-variant)] focus-visible:rounded focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)]">
            <span className="font-semibold text-[var(--color-on-surface)]">{name}</span>
            {code ? <code className="ml-2 break-all text-[11px] text-[var(--color-on-surface-variant)]">{code}</code> : null}
            <span className="ml-2 text-[11px] text-[var(--color-on-surface-variant)]">· {t("chat.applications.preview.toolCount", { count: toolCount })}</span>
            {typeof domain.hasMutationTools === "boolean" ? <span className="ml-2 text-[11px] text-[var(--color-on-surface-variant)]">· {t(`chat.applications.preview.${domain.hasMutationTools ? "canModify" : "readOnly"}`)}</span> : null}
            {typeof domain.description === "string" && domain.description ? <span className="mt-1 block pl-4 leading-5 text-[var(--color-on-surface-variant)]">{domain.description}</span> : null}
          </summary>
          {sampleTools.length ? <div className="pb-1 pl-4 pt-2">
            <p className="mb-1.5 text-[11px] text-[var(--color-on-surface-variant)]">{t("chat.applications.preview.sampleTools")}</p>
            <div className="flex flex-wrap gap-1">{sampleTools.map((tool, toolIndex) => <code className="break-all rounded bg-[var(--color-surface-container)] px-1.5 py-0.5 text-[11px]" key={`${tool}:${toolIndex}`}>{tool}</code>)}</div>
          </div> : null}
        </details>;
      })}
    </div>
  </section>;
}

function ApplicationMessagePart({ part }: { part: RecordValue }) {
  const { t } = useTranslation("pages");
  const text = typeof part.text === "string" ? part.text : null;
  if (text) {
    const directory = taggedContent(text, "available_tool_domains");
    if (directory !== null) {
      const domains = parse(directory);
      if (Array.isArray(domains) && domains.every(record)) return <ToolDomainsPreview domains={domains} />;
    }
    const execution = taggedContent(text, "application-execution-context");
    if (execution !== null) {
      const jsonStart = execution.indexOf("{");
      const scope = jsonStart >= 0 ? parse(execution.slice(jsonStart)) : null;
      if (record(scope)) return <section className="space-y-2">
        <h3 className="text-xs font-semibold">{t("chat.applications.preview.executionLocation")}</h3>
        <ApplicationScopePreview scope={scope} />
      </section>;
    }
  }
  return <MessagePart part={part} />;
}

function ApplicationPreview({ value }: { value: unknown }) {
  const { t } = useTranslation("pages");
  const messages = record(value) ? records(value.messages) : [];
  if (messages.length) return <div className="space-y-4">{messages.flatMap((message) => records(message.parts)).map((part, index) => <ApplicationMessagePart key={index} part={part} />)}</div>;
  const scope = record(value) && record(value.executionScope) ? value.executionScope : null;
  return scope ? <ApplicationScopePreview scope={scope} /> : <Empty label={t("chat.applications.noSnapshot")} />;
}

export function ContextDetailPreview({ block, value, onOpenPrompt }: { block: ContextDetailBlock; value: unknown; onOpenPrompt?: (code: string) => void }) {
  if (block === "toolSchema") return <ToolPreview value={value} />;
  if (block === "applicationContext") return <ApplicationPreview value={value} />;
  return <MessagesPreview value={value} history={block === "history"} systemPrompt={block === "systemPrompt"} onOpenPrompt={onOpenPrompt} />;
}
