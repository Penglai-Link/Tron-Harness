import {
  Plus,
  Trash2,
} from "lucide-react";

import type {
  McpAuthKind,
  McpServerTransportKind,
  PlatformMcpServerDefinitionPayload,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
} from "@/domain/mcp/model";
import {
  Button,
  Field,
  InfoTooltip,
  SelectInput,
  TextArea,
  TextInput,
} from "@/shared";
import { SegmentedTabs } from "@/shared/ui/Tabs";
import { SelectItem } from "@/components/ui/select";

export interface KeyValueRow {
  key: string;
  value: string;
}

export interface StringRow {
  value: string;
}

export interface PlatformMcpServerFormState {
  transportKind: McpServerTransportKind;
  serverCode: string;
  name: string;
  description: string;
  url: string;
  authKind: McpAuthKind;
  authTokenRef: string;
  headers: KeyValueRow[];
  headersVariableRefs: KeyValueRow[];
  command: string;
  args: StringRow[];
  envVariableRefs: KeyValueRow[];
  envPassthroughRefs: StringRow[];
  workingDirectory: string;
}

const TRANSPORT_TABS = [
  { label: "Streamable HTTP", value: "streamable_http" },
  { label: "STDIO", value: "stdio" },
] as const;

export function emptyPlatformMcpServerForm(transportKind: McpServerTransportKind = "streamable_http"): PlatformMcpServerFormState {
  return {
    transportKind,
    serverCode: "",
    name: "",
    description: "",
    url: "",
    authKind: "none",
    authTokenRef: "",
    headers: [{ key: "", value: "" }],
    headersVariableRefs: [{ key: "", value: "" }],
    command: "",
    args: [{ value: "" }],
    envVariableRefs: [{ key: "", value: "" }],
    envPassthroughRefs: [{ value: "" }],
    workingDirectory: "",
  };
}

export function platformMcpServerFormFromRecord(record: PlatformMcpServerRecord): PlatformMcpServerFormState {
  return {
    transportKind: record.transportKind,
    serverCode: record.serverCode,
    name: record.name,
    description: record.description,
    url: record.url,
    authKind: record.authKind,
    authTokenRef: record.authPayload?.tokenRef ?? "",
    headers: mapObjectToKeyValueRows(record.headers),
    headersVariableRefs: mapObjectToKeyValueRows(record.headersVariableRefs),
    command: record.command,
    args: mapListToRows(record.args),
    envVariableRefs: mapObjectToKeyValueRows(record.envVariableRefs),
    envPassthroughRefs: mapListToRows(record.envPassthroughRefs),
    workingDirectory: record.workingDirectory ?? "",
  };
}

export function cloneFormForTransport(
  form: PlatformMcpServerFormState,
  transportKind: McpServerTransportKind,
): PlatformMcpServerFormState {
  if (transportKind === form.transportKind) {
    return form;
  }

  return transportKind === "streamable_http"
    ? {
      ...form,
      transportKind,
      url: form.url,
      authKind: "none",
      authTokenRef: "",
      headers: ensureNonEmptyKeyValueRows(form.headers),
      headersVariableRefs: ensureNonEmptyKeyValueRows(form.headersVariableRefs),
      command: "",
      args: [{ value: "" }],
      envVariableRefs: [{ key: "", value: "" }],
      envPassthroughRefs: [{ value: "" }],
      workingDirectory: "",
    }
    : {
      ...form,
      transportKind,
      url: "",
      authKind: "none",
      authTokenRef: "",
      headers: [{ key: "", value: "" }],
      headersVariableRefs: [{ key: "", value: "" }],
      command: form.command,
      args: ensureNonEmptyStringRows(form.args),
      envVariableRefs: ensureNonEmptyKeyValueRows(form.envVariableRefs),
      envPassthroughRefs: ensureNonEmptyStringRows(form.envPassthroughRefs),
      workingDirectory: form.workingDirectory,
    };
}

export function platformMcpServerCreatePayload(form: PlatformMcpServerFormState): PlatformMcpServerPayload {
  return {
    serverCode: form.serverCode.trim(),
    ...platformMcpServerDefinitionPayload(form),
  };
}

export function platformMcpServerDefinitionPayload(form: PlatformMcpServerFormState): PlatformMcpServerDefinitionPayload {
  return {
    name: form.name.trim(),
    description: normalizeDescription(form.description),
    transportKind: form.transportKind,
    url: form.transportKind === "streamable_http" ? form.url.trim() : "",
    authKind: form.transportKind === "streamable_http" ? form.authKind : "none",
    authPayload: resolveAuthPayload(form),
    headers: form.transportKind === "streamable_http" ? keyValueRowsToMap(form.headers) : {},
    headersVariableRefs: form.transportKind === "streamable_http" ? keyValueRowsToMap(form.headersVariableRefs) : {},
    command: form.transportKind === "stdio" ? form.command.trim() : "",
    args: form.transportKind === "stdio" ? stringRowsToList(form.args) : [],
    envVariableRefs: form.transportKind === "stdio" ? keyValueRowsToMap(form.envVariableRefs) : {},
    envPassthroughRefs: form.transportKind === "stdio" ? stringRowsToList(form.envPassthroughRefs) : [],
    workingDirectory: form.transportKind === "stdio" ? form.workingDirectory.trim() : "",
  };
}

export function validatePlatformMcpServerForm(
  form: PlatformMcpServerFormState,
  options: { keyEditable: boolean },
) {
  if (options.keyEditable && !form.serverCode.trim()) {
    return "Server code is required";
  }

  if (!form.name.trim()) {
    return "Name is required";
  }

  if (form.transportKind === "streamable_http") {
    if (!form.url.trim()) {
      return "URL is required";
    }
    if (form.authKind === "bearer_static" && !form.authTokenRef.trim()) {
      return "Bearer token variable ref is required";
    }
  } else if (!form.command.trim()) {
    return "Command is required";
  }

  return null;
}

export function formatPlatformMcpTransportLabel(transportKind: McpServerTransportKind) {
  return transportKind === "stdio" ? "STDIO" : "Streamable HTTP";
}

export function formatPlatformMcpEndpoint(record: PlatformMcpServerRecord) {
  if (record.transportKind === "stdio") {
    const args = record.args.length ? ` ${record.args.join(" ")}` : "";
    return `${record.command}${args}`.trim() || "STDIO";
  }
  return record.url;
}

export function platformMcpServerStateTone(status: PlatformMcpServerRecord["lastHealthStatus"]) {
  switch (status) {
    case "HEALTHY":
      return "success" as const;
    case "UNHEALTHY":
      return "warning" as const;
    default:
      return "info" as const;
  }
}

export function platformMcpServerEnabledTone(enabled: boolean) {
  return enabled ? "success" as const : "info" as const;
}

export function platformMcpServerTransportTabsValue(transportKind: McpServerTransportKind) {
  return transportKind;
}

export function PlatformMcpServerFormFields({
  form,
  keyEditable,
  onChange,
}: {
  form: PlatformMcpServerFormState;
  keyEditable: boolean;
  onChange: (next: PlatformMcpServerFormState) => void;
}) {
  function updateForm(next: Partial<PlatformMcpServerFormState>) {
    onChange({ ...form, ...next });
  }

  function updateTransportKind(transportKind: McpServerTransportKind) {
    onChange(cloneFormForTransport(form, transportKind));
  }

  return (
    <div className="space-y-6">
      <Field label="Transport">
        <SegmentedTabs
          value={platformMcpServerTransportTabsValue(form.transportKind)}
          onValueChange={(value) => updateTransportKind(value as McpServerTransportKind)}
          items={TRANSPORT_TABS}
        />
      </Field>

      <div className="grid gap-5 md:grid-cols-2">
        <Field
          label="Server Code"
          hint={keyEditable
            ? "Lowercase letters, digits, underscore, and hyphen only."
            : "Server code is immutable after creation."}
        >
          <TextInput
            aria-label="Platform MCP server code"
            disabled={!keyEditable}
            value={form.serverCode}
            onChange={(event) => updateForm({ serverCode: event.target.value })}
          />
        </Field>
        <Field label="Name" hint="Human-readable name shown in registry lists.">
          <TextInput
            aria-label="Platform MCP server name"
            value={form.name}
            onChange={(event) => updateForm({ name: event.target.value })}
          />
        </Field>
      </div>

      <Field label="Description" hint="Optional operator-facing description.">
        <TextArea
          aria-label="Platform MCP server description"
          value={form.description}
          onChange={(event) => updateForm({ description: event.target.value })}
        />
      </Field>

      {form.transportKind === "streamable_http" ? (
        <div className="space-y-5">
          <div className="grid gap-5 md:grid-cols-2">
            <Field label="URL" hint="Streamable HTTP endpoint, for example https://example.com/mcp.">
              <TextInput
                aria-label="Platform MCP server URL"
                value={form.url}
                onChange={(event) => updateForm({ url: event.target.value })}
              />
            </Field>
            <Field label="Auth Kind" hint="Bearer static reads the token from Variable Management using {{var-name}}.">
              <SelectInput
                aria-label="Platform MCP auth kind"
                value={form.authKind}
                onValueChange={(event) => {
                  const authKind = event as McpAuthKind;
                  onChange({
                    ...form,
                    authKind,
                    authTokenRef: authKind === "bearer_static" ? form.authTokenRef : "",
                  });
                }}
              >
                <SelectItem value="none">None</SelectItem>
                <SelectItem value="bearer_static">Bearer static</SelectItem>
              </SelectInput>
            </Field>
          </div>

          {form.authKind === "bearer_static" ? (
            <Field
              label="Bearer token variable ref"
              hint="Enter a Variable Management reference in {{var-name}} form, such as {{MCP_TOKEN}}."
            >
              <TextInput
                aria-label="Bearer token variable ref"
                value={form.authTokenRef}
                onChange={(event) => updateForm({ authTokenRef: event.target.value })}
              />
            </Field>
          ) : null}

          <KeyValueListSection
            label="Headers"
            addLabel="header"
            hint="Value can be a static string. If it should come from Variable Management, use {{var-name}}."
            rows={form.headers}
            onChange={(rows) => updateForm({ headers: rows })}
          />

          <KeyValueListSection
            label="Header variable refs"
            addLabel="header ref"
            hint="Use {{var-name}} in the Value field so the header value resolves from Variable Management at runtime."
            rows={form.headersVariableRefs}
            onChange={(rows) => updateForm({ headersVariableRefs: rows })}
          />
        </div>
      ) : (
        <div className="space-y-5">
          <Field label="Command" hint="Command used to launch the stdio MCP server.">
            <TextInput
              aria-label="Platform MCP stdio command"
              value={form.command}
              onChange={(event) => updateForm({ command: event.target.value })}
            />
          </Field>

          <StringListSection
            label="Arguments"
            addLabel="argument"
            hint="Enter one argument per row. Do not paste the full command line into a single row."
            placeholder="Argument"
            rows={form.args}
            onChange={(rows) => updateForm({ args: rows })}
          />

          <KeyValueListSection
            label="Environment variables"
            addLabel="environment variable"
            hint="Use {{var-name}} in the Value field so the env value resolves from Variable Management at runtime."
            rows={form.envVariableRefs}
            onChange={(rows) => updateForm({ envVariableRefs: rows })}
          />

          <StringListSection
            label="Environment variable passthrough"
            addLabel="passthrough variable"
            hint="These environment variables are forwarded from the current process."
            rows={form.envPassthroughRefs}
            onChange={(rows) => updateForm({ envPassthroughRefs: rows })}
          />

          <Field label="Working directory" hint="Optional working directory for the spawned stdio process.">
            <TextInput
              aria-label="Platform MCP working directory"
              value={form.workingDirectory}
              onChange={(event) => updateForm({ workingDirectory: event.target.value })}
            />
          </Field>
        </div>
      )}
    </div>
  );
}

function KeyValueListSection({
  addLabel,
  hint,
  label,
  rows,
  onChange,
}: {
  addLabel: string;
  hint: string;
  label: string;
  rows: KeyValueRow[];
  onChange: (rows: KeyValueRow[]) => void;
}) {
  return (
    <div className="grid gap-2">
      <div className="flex items-center gap-1.5">
        <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
        <InfoTooltip ariaLabel={`${label} help`}>
          <p>{hint}</p>
        </InfoTooltip>
      </div>
      <div className="space-y-2">
        {rows.map((row, index) => (
          <div key={`${label}-${index}`} className="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
            <TextInput
              aria-label={`${label} key ${index + 1}`}
              placeholder="Key"
              value={row.key}
              onChange={(event) => onChange(rows.map((item, rowIndex) => (
                rowIndex === index ? { ...item, key: event.target.value } : item
              )))}
            />
            <TextInput
              aria-label={`${label} value ${index + 1}`}
              placeholder="Value"
              value={row.value}
              onChange={(event) => onChange(rows.map((item, rowIndex) => (
                rowIndex === index ? { ...item, value: event.target.value } : item
              )))}
            />
            <Button
              size="icon"
              variant="ghost"
              aria-label={`Remove ${label.toLowerCase()} row ${index + 1}`}
              onClick={() => onChange(rows.length > 1 ? rows.filter((_, rowIndex) => rowIndex !== index) : [{ key: "", value: "" }])}
            >
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ))}
      </div>
      <Button
        variant="secondary"
        onClick={() => onChange([...rows, { key: "", value: "" }])}
      >
        <Plus size={16} aria-hidden="true" />
        Add {addLabel}
      </Button>
    </div>
  );
}

function StringListSection({
  addLabel,
  hint,
  label,
  placeholder = "Value",
  rows,
  onChange,
}: {
  addLabel: string;
  hint: string;
  label: string;
  placeholder?: string;
  rows: StringRow[];
  onChange: (rows: StringRow[]) => void;
}) {
  return (
    <div className="grid gap-2">
      <div className="flex items-center gap-1.5">
        <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
        <InfoTooltip ariaLabel={`${label} help`}>
          <p>{hint}</p>
        </InfoTooltip>
      </div>
      <div className="space-y-2">
        {rows.map((row, index) => (
          <div key={`${label}-${index}`} className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto]">
            <TextInput
              aria-label={`${label} item ${index + 1}`}
              placeholder={placeholder}
              value={row.value}
              onChange={(event) => onChange(rows.map((item, rowIndex) => (
                rowIndex === index ? { ...item, value: event.target.value } : item
              )))}
            />
            <Button
              size="icon"
              variant="ghost"
              aria-label={`Remove ${label.toLowerCase()} item ${index + 1}`}
              onClick={() => onChange(rows.length > 1 ? rows.filter((_, rowIndex) => rowIndex !== index) : [{ value: "" }])}
            >
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ))}
      </div>
      <Button variant="secondary" onClick={() => onChange([...rows, { value: "" }])}>
        <Plus size={16} aria-hidden="true" />
        Add {addLabel}
      </Button>
    </div>
  );
}

function ensureNonEmptyKeyValueRows(rows: KeyValueRow[]) {
  return rows.length > 0 ? rows : [{ key: "", value: "" }];
}

function ensureNonEmptyStringRows(rows: StringRow[]) {
  return rows.length > 0 ? rows : [{ value: "" }];
}

function mapObjectToKeyValueRows(values: Record<string, string>): KeyValueRow[] {
  const rows = Object.entries(values).map(([key, value]) => ({
    key,
    value,
  }));
  return rows.length > 0 ? rows : [{ key: "", value: "" }];
}

function mapListToRows(values: string[]): StringRow[] {
  const rows = values.map((value) => ({ value }));
  return rows.length > 0 ? rows : [{ value: "" }];
}

function keyValueRowsToMap(rows: KeyValueRow[]) {
  return Object.fromEntries(
    rows
      .map((row) => [row.key.trim(), row.value.trim()] as const)
      .filter(([key, value]) => key.length > 0 && value.length > 0),
  );
}

function stringRowsToList(rows: StringRow[]) {
  return rows
    .map((row) => row.value.trim())
    .filter((value, index, values) => value.length > 0 && values.indexOf(value) === index);
}

function resolveAuthPayload(form: PlatformMcpServerFormState) {
  if (form.transportKind !== "streamable_http" || form.authKind !== "bearer_static") {
    return null;
  }

  const tokenRef = form.authTokenRef.trim();
  return tokenRef ? { tokenRef } : null;
}

function normalizeDescription(value: string) {
  return value.trim();
}
