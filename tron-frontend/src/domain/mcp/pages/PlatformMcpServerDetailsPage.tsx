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
import { useTranslation } from "react-i18next";
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

function ReadValue({ children }: { children?: ReactNode }) {
  const { t } = useTranslation(["pages"]);
  return (
    <div className="min-h-6 text-sm leading-6 text-[var(--color-on-surface)] break-words">
      {children || t("pages:mcp.form.none", "None")}
    </div>
  );
}

function ReadCodeBlock({ children }: { children: string[] }) {
  if (!children.length) {
    return <ReadValue />;
  }

  return (
    <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3">
      <pre className="whitespace-pre-wrap break-words font-mono text-sm leading-6 text-[var(--color-on-surface)]">{children.join("\n")}</pre>
    </div>
  );
}

function ReadKeyValueBlock({ entries }: { entries: Array<[string, string]> }) {
  if (!entries.length) {
    return <ReadValue />;
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
  const { t } = useTranslation(["pages"]);
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
          toast.error(error instanceof Error ? error.message : t("pages:mcp.errors.load", "Platform MCP server could not be loaded"));
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
          breadcrumbs={[{ label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" }, { label: t("pages:variables.loading", "Loading") }]}
          title={t("pages:mcp.detailTitle", "Platform MCP Server Detail")}
        />
        <SurfaceCard>{t("pages:mcp.loading", "Loading platform MCP server...")}</SurfaceCard>
      </>
    );
  }

  if (!record) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" }, { label: t("pages:variables.notFound", "Not Found") }]}
          title={t("pages:mcp.detailTitle", "Platform MCP Server Detail")}
        />
        <SurfaceCard>{t("pages:mcp.notFound", "Platform MCP server not found.")}</SurfaceCard>
      </>
    );
  }

  const detailHeader = record.transportKind === "streamable_http"
    ? [
      [t("pages:mcp.detail.url", "URL"), record.url],
      [t("pages:mcp.detail.authKind", "Auth Kind"), authKindLabel(record.authKind)],
      ...(record.authKind === "bearer_static" ? [[t("pages:mcp.detail.tokenRef", "Bearer token variable ref"), record.authPayload?.tokenRef ?? ""]] : []),
    ]
    : [
      [t("pages:mcp.form.command", "Command"), record.command],
      [t("pages:mcp.form.workingDirectory", "Working directory"), record.workingDirectory ?? ""],
    ];

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" },
          { label: record.name },
        ]}
        title={record.name}
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate(`/settings/mcp-servers/edit?serverCode=${encodeURIComponent(record.serverCode)}`)}>
              <Edit3 size={16} aria-hidden="true" />
              {t("pages:mcp.actions.edit", "Edit")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        <SurfaceCard className="space-y-6">
          <div className="grid gap-4 md:grid-cols-3">
            <Field label={t("pages:mcp.form.serverCode", "Server Code")}>
              <ReadValue>
                <code>{record.serverCode}</code>
              </ReadValue>
            </Field>
            <Field label={t("pages:mcp.form.transport", "Transport")}>
              <ReadValue>{formatPlatformMcpTransportLabel(record.transportKind)}</ReadValue>
            </Field>
            <Field label={t("pages:mcp.detail.endpoint", "Endpoint")}>
              <ReadValue>{formatPlatformMcpEndpoint(record)}</ReadValue>
            </Field>
            <Field label={t("pages:mcp.detail.enabled", "Enabled")}>
              <Badge tone={platformMcpServerEnabledTone(record.enabled)}>
                {record.enabled ? t("pages:mcp.detail.enabledValue", "Enabled") : t("pages:mcp.detail.disabledValue", "Disabled")}
              </Badge>
            </Field>
            <Field label={t("pages:mcp.detail.health", "Health")}>
              <Badge tone={platformMcpServerStateTone(record.lastHealthStatus)}>
                {healthLabel(record.lastHealthStatus)}
              </Badge>
            </Field>
            <Field label={t("pages:mcp.detail.created", "Created")}>
              <ReadValue>
                {formatDateTime(record.createdAt, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </ReadValue>
            </Field>
            <Field label={t("pages:mcp.detail.updated", "Updated")}>
              <ReadValue>
                {formatDateTime(record.updatedAt, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </ReadValue>
            </Field>
          </div>

          <div className="space-y-3">
            <Field label={t("pages:mcp.form.description", "Description")}>
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
                <Field label={t("pages:mcp.form.headers", "Headers")}>
                  <ReadKeyValueBlock entries={Object.entries(record.headers)} />
                </Field>
                <Field label={t("pages:mcp.form.headerRefs", "Header variable refs")}>
                  <ReadKeyValueBlock entries={Object.entries(record.headersVariableRefs)} />
                </Field>
              </div>
            </div>
          ) : (
            <div className="space-y-6">
              <Field label={t("pages:mcp.form.arguments", "Arguments")}>
                <ReadCodeBlock children={record.args} />
              </Field>
              <Field label={t("pages:mcp.form.environment", "Environment variables")}>
                <ReadKeyValueBlock entries={Object.entries(record.envVariableRefs)} />
              </Field>
              <Field label={t("pages:mcp.form.passthrough", "Environment variable passthrough")}>
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
