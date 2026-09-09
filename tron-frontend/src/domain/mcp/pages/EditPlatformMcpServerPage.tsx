import {
  useEffect,
  useState,
} from "react";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";
import { useTranslation } from "react-i18next";
import {
  Save,
  ShieldCheck,
} from "lucide-react";

import {
  fetchPlatformMcpServers,
  updatePlatformMcpServer,
  verifyPlatformMcpServer,
} from "@/domain/mcp/api";
import {
  listPlatformMcpServersMock,
  updatePlatformMcpServerMock,
  verifyPlatformMcpServerMock,
} from "@/domain/mcp/mock";
import type {
  PlatformMcpServerRecord,
} from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useToast,
} from "@/shared";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

import {
  emptyPlatformMcpServerForm,
  platformMcpServerDefinitionPayload,
  platformMcpServerFormFromRecord,
  PlatformMcpServerFormFields,
  platformMcpServerStateTone,
  validatePlatformMcpServerForm,
} from "./mcpServerPageHelpers";

function healthLabel(status: PlatformMcpServerRecord["lastHealthStatus"]) {
  return status.replaceAll("_", " ");
}

function VerificationResultCard({
  record,
}: {
  record: PlatformMcpServerRecord;
}) {
  const { formatDateTime } = useLocaleFormatters();

  return (
    <SurfaceCard className="space-y-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Verification result</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            Current configuration verified against the platform MCP runtime.
          </p>
        </div>
        <Badge variant={platformMcpServerStateTone(record.lastHealthStatus)}>
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
  );
}

export function EditPlatformMcpServerPage() {
  const { t } = useTranslation(["pages"]);
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("mcp");
  const serverCode = searchParams.get("serverCode") ?? "";
  const [record, setRecord] = useState<PlatformMcpServerRecord | null>(null);
  const [form, setForm] = useState(emptyPlatformMcpServerForm());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [verificationResult, setVerificationResult] = useState<PlatformMcpServerRecord | null>(null);

  const validationError = validatePlatformMcpServerForm(form, { keyEditable: false });

  function updateForm(next: typeof form) {
    setForm(next);
    setVerificationResult(null);
  }

  useEffect(() => {
    if (!serverCode) return;

    let active = true;

    async function loadServer() {
      setLoading(true);
      try {
        const records = dataSource === "api"
          ? await fetchPlatformMcpServers()
          : listPlatformMcpServersMock();
        const nextRecord = records.find((item) => item.serverCode === serverCode) ?? null;
        if (!active) return;
        setRecord(nextRecord);
        if (nextRecord) {
          setForm(platformMcpServerFormFromRecord(nextRecord));
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

  async function verifyDraft() {
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setVerifying(true);
    try {
      const payload = platformMcpServerDefinitionPayload(form);
      const verified = dataSource === "api"
        ? await verifyPlatformMcpServer(record?.serverCode ?? serverCode, {
          definition: payload,
        })
        : verifyPlatformMcpServerMock(record?.serverCode ?? serverCode, {
          definition: payload,
        });
      setVerificationResult(verified);
      toast.success(t("pages:mcp.messages.verified", "{{name}} verified", { name: verified.name }));
    } catch (error) {
      setVerificationResult(null);
      toast.error(error instanceof Error ? error.message : t("pages:mcp.errors.verify", "Verify failed"));
    } finally {
      setVerifying(false);
    }
  }

  async function saveServer() {
    if (!record) return;
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSubmitting(true);
    try {
      const payload = platformMcpServerDefinitionPayload(form);
      const saved = dataSource === "api"
        ? await updatePlatformMcpServer(record.serverCode, payload)
        : updatePlatformMcpServerMock(record.serverCode, payload);
      setRecord(saved);
      setForm(platformMcpServerFormFromRecord(saved));
      setVerificationResult(null);
      toast.success(t("pages:mcp.messages.updated", "{{name}} updated", { name: saved.name }));
      navigate(`/settings/mcp-servers/detail?serverCode=${encodeURIComponent(saved.serverCode)}`, { replace: true });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:mcp.errors.save", "Platform MCP server could not be saved"));
    } finally {
      setSubmitting(false);
    }
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" }, { label: t("pages:variables.loading", "Loading") }]}
          title={t("pages:mcp.editTitle", "Edit Platform MCP Server")}
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
          title={t("pages:mcp.editTitle", "Edit Platform MCP Server")}
        />
        <SurfaceCard>{t("pages:mcp.notFound", "Platform MCP server not found.")}</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" },
          { label: record.name, href: `/settings/mcp-servers/edit?serverCode=${encodeURIComponent(record.serverCode)}` },
          { label: t("pages:mcp.actions.edit", "Edit") },
        ]}
        title={t("pages:mcp.editRecordTitle", "Edit {{name}}", { name: record.name })}
        actions={(
          <>
            <Button variant="secondary" disabled={submitting || verifying} onClick={() => navigate("/settings/mcp-servers")}>{t("pages:mcp.actions.cancel", "Cancel")}</Button>
            <Button
              variant="secondary"
              disabled={submitting || verifying || Boolean(validationError)}
              aria-busy={verifying}
              onClick={() => void verifyDraft()}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {verifying ? t("pages:mcp.actions.verifying", "Verifying...") : t("pages:mcp.actions.verify", "Verify")}
            </Button>
            <Button
              disabled={submitting || verifying || Boolean(validationError)}
              aria-busy={submitting}
              onClick={() => void saveServer()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? t("pages:mcp.actions.saving", "Saving...") : t("pages:mcp.actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        <SurfaceCard className="space-y-6">
          <PlatformMcpServerFormFields form={form} keyEditable={false} onChange={updateForm} />
        </SurfaceCard>

        {verificationResult ? <VerificationResultCard record={verificationResult} /> : null}
      </div>
    </>
  );
}
