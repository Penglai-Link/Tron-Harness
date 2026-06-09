import {
  useEffect,
  useState,
} from "react";
import type { ReactNode } from "react";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Edit3,
} from "lucide-react";

import {
  fetchPlatformMcpServer,
} from "@/domain/mcp/api";
import {
  getPlatformMcpServerMock,
} from "@/domain/mcp/mock";
import type {
  McpAuthKind,
  PlatformMcpServerRecord,
} from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  Button,
  Field,
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useToast,
} from "@/shared";

import {
  formatPlatformMcpEndpoint,
  formatPlatformMcpTransportLabel,
  platformMcpServerEnabledTone,
  platformMcpServerStateTone,
} from "./mcpServerPageHelpers";

function healthLabel(status: PlatformMcpServerRecord["lastHealthStatus"]) {
  return status.replaceAll("_", " ");
}

function authKindLabel(authKind: McpAuthKind) {
  return authKind === "bearer_static" ? "Bearer static" : "None";
}

function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-6 text-sm leading-6 text-[var(--color-on-surface)] break-words">
      {children || "None"}
    </div>
  );
}

function ReadCodeBlock({ children }: { children: string[] }) {
  if (!children.length) {
    return <ReadValue>None</ReadValue>;
  }

  return (
    <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
      <pre className="whitespace-pre-wrap break-words font-mono text-sm leading-6 text-[var(--color-on-surface)]">{children.join("\n")}</pre>
    </div>
  );
}

function ReadKeyValueBlock({ entries }: { entries: Array<[string, string]> }) {
  if (!entries.length) {
    return <ReadValue>None</ReadValue>;
  }

  return (
    <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
      <div className="space-y-1 font-mono text-sm leading-6 text-[var(--color-on-surface)]">
        {entries.map(([key, value]) => (
          <div key={key} className="break-words">
            {key} = {value}
          </div>
        ))}
      </div>
    </div>
  );
}

export function PlatformMcpServerDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("mcp");
  const serverCode = searchParams.get("serverCode") ?? "";
  const [record, setRecord] = useState<PlatformMcpServerRecord | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!serverCode) return;

    let active = true;

    async function loadServer() {
      setLoading(true);
      try {
        const nextRecord = dataSource === "api"
          ? await fetchPlatformMcpServer(serverCode)
          : getPlatformMcpServerMock(serverCode);
        if (active) {
          setRecord(nextRecord);
        }
      } catch (error) {
        if (active) {
          toast.error(error instanceof Error ? error.message : "Platform MCP server could not be loaded");
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    }

    void loadServer();

    return () => {
      active = false;
    };
  }, [dataSource, serverCode, toast]);

  if (!serverCode) {
    return <Navigate replace to="/settings/mcp-servers" />;
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "MCP Servers", href: "/settings/mcp-servers" }, { label: "Loading" }]}
          title="Platform MCP Server Detail"
        />
        <SurfaceCard>Loading platform MCP server...</SurfaceCard>
      </>
    );
  }

  if (!record) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "MCP Servers", href: "/settings/mcp-servers" }, { label: "Not Found" }]}
          title="Platform MCP Server Detail"
        />
        <SurfaceCard>Platform MCP server not found.</SurfaceCard>
      </>
    );
  }

  const detailHeader = record.transportKind === "streamable_http"
    ? [
      ["URL", record.url],
      ["Auth Kind", authKindLabel(record.authKind)],
      ...(record.authKind === "bearer_static" ? [["Bearer token variable ref", record.authPayload?.tokenRef ?? ""]] : []),
    ]
    : [
      ["Command", record.command],
      ["Working directory", record.workingDirectory ?? ""],
    ];

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: "MCP Servers", href: "/settings/mcp-servers" },
          { label: record.name },
        ]}
        title={record.name}
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate(`/settings/mcp-servers/edit?serverCode=${encodeURIComponent(record.serverCode)}`)}>
              <Edit3 size={16} aria-hidden="true" />
              Edit
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        <SurfaceCard className="space-y-6">
          <div className="grid gap-4 md:grid-cols-3">
            <Field label="Server Code">
              <ReadValue>
                <code>{record.serverCode}</code>
              </ReadValue>
            </Field>
            <Field label="Transport">
              <ReadValue>{formatPlatformMcpTransportLabel(record.transportKind)}</ReadValue>
            </Field>
            <Field label="Endpoint">
              <ReadValue>{formatPlatformMcpEndpoint(record)}</ReadValue>
            </Field>
            <Field label="Enabled">
              <Badge tone={platformMcpServerEnabledTone(record.enabled)}>
                {record.enabled ? "Enabled" : "Disabled"}
              </Badge>
            </Field>
            <Field label="Health">
              <Badge tone={platformMcpServerStateTone(record.lastHealthStatus)}>
                {healthLabel(record.lastHealthStatus)}
              </Badge>
            </Field>
            <Field label="Created">
              <ReadValue>
                {formatDateTime(record.createdAt, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </ReadValue>
            </Field>
            <Field label="Updated">
              <ReadValue>
                {formatDateTime(record.updatedAt, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </ReadValue>
            </Field>
          </div>

          <div className="space-y-3">
            <Field label="Description">
              <ReadValue>{record.description}</ReadValue>
            </Field>
          </div>
        </SurfaceCard>

        <SurfaceCard className="space-y-6">
          <div className="grid gap-6 md:grid-cols-2">
            {detailHeader.map(([label, value]) => (
              <Field key={label} label={label}>
                <ReadValue>{value}</ReadValue>
              </Field>
            ))}
          </div>

          {record.transportKind === "streamable_http" ? (
            <div className="space-y-6">
              <div className="grid gap-6 md:grid-cols-2">
                <Field label="Headers">
                  <ReadKeyValueBlock entries={Object.entries(record.headers)} />
                </Field>
                <Field label="Header variable refs">
                  <ReadKeyValueBlock entries={Object.entries(record.headersVariableRefs)} />
                </Field>
              </div>
            </div>
          ) : (
            <div className="space-y-6">
              <Field label="Arguments">
                <ReadCodeBlock children={record.args} />
              </Field>
              <Field label="Environment variables">
                <ReadKeyValueBlock entries={Object.entries(record.envVariableRefs)} />
              </Field>
              <Field label="Environment variable passthrough">
                <ReadCodeBlock children={record.envPassthroughRefs} />
              </Field>
            </div>
          )}
        </SurfaceCard>

        <SurfaceCard className="space-y-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold">Verification result</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                Current configuration verified against the platform MCP runtime.
              </p>
            </div>
            <Badge tone={platformMcpServerStateTone(record.lastHealthStatus)}>
              {healthLabel(record.lastHealthStatus)}
            </Badge>
          </div>
          <p className="text-sm text-[var(--color-on-surface)]">
            {record.lastHealthMessage ?? "No issues reported."}
          </p>
          {record.lastHealthCheckedAt ? (
            <p className="text-xs text-[var(--color-on-surface-variant)]">
              Checked {formatDateTime(record.lastHealthCheckedAt, {
                dateStyle: "medium",
                timeStyle: "short",
              })}
            </p>
          ) : null}
        </SurfaceCard>
      </div>
    </>
  );
}
