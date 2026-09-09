import {
  useState,
} from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import {
  Save,
  ShieldCheck,
} from "lucide-react";

import {
  createPlatformMcpServer,
  verifyPlatformMcpServerDraft,
} from "@/domain/mcp/api";
import {
  createPlatformMcpServerMock,
  verifyPlatformMcpServerDraftMock,
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
  platformMcpServerCreatePayload,
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
  const { t } = useTranslation(["pages"]);
  const { formatDateTime } = useLocaleFormatters();

  return (
    <SurfaceCard className="space-y-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{t("pages:mcp.verification.title", "Verification result")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {t("pages:mcp.verification.description", "Draft configuration verified against the platform MCP runtime.")}
          </p>
        </div>
        <Badge variant={platformMcpServerStateTone(record.lastHealthStatus)}>
          {healthLabel(record.lastHealthStatus)}
        </Badge>
      </div>

      <p className="text-sm text-[var(--color-on-surface)]">
        {record.lastHealthMessage ?? t("pages:mcp.verification.noIssues", "No issues reported.")}
      </p>

      {record.lastHealthCheckedAt ? (
        <p className="text-xs text-[var(--color-on-surface-variant)]">
          {t("pages:mcp.verification.checked", "Checked")} {formatDateTime(record.lastHealthCheckedAt, {
            dateStyle: "medium",
            timeStyle: "short",
          })}
        </p>
      ) : null}
    </SurfaceCard>
  );
}

export function CreatePlatformMcpServerPage() {
  const { t } = useTranslation(["pages"]);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("mcp");
  const [form, setForm] = useState(emptyPlatformMcpServerForm());
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [verificationResult, setVerificationResult] = useState<PlatformMcpServerRecord | null>(null);

  const validationError = validatePlatformMcpServerForm(form, { keyEditable: true });

  function updateForm(next: typeof form) {
    setForm(next);
    setVerificationResult(null);
  }

  async function verifyDraft() {
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setVerifying(true);
    try {
      const payload = platformMcpServerCreatePayload(form);
      const verified = dataSource === "api"
        ? await verifyPlatformMcpServerDraft({
          definition: payload,
        })
        : verifyPlatformMcpServerDraftMock({
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
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSubmitting(true);
    try {
      const payload = platformMcpServerCreatePayload(form);
      const saved = dataSource === "api"
        ? await createPlatformMcpServer(payload)
        : createPlatformMcpServerMock(payload);
      toast.success(t("pages:mcp.messages.created", "{{name}} created", { name: saved.name }));
      navigate(`/settings/mcp-servers/detail?serverCode=${encodeURIComponent(saved.serverCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:mcp.errors.create", "Platform MCP server could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: t("pages:mcp.title", "MCP Servers"), href: "/settings/mcp-servers" },
          { label: t("pages:mcp.new", "New") },
        ]}
        title={t("pages:mcp.createTitle", "Connect MCP server")}
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/mcp-servers")}>{t("pages:mcp.actions.cancel", "Cancel")}</Button>
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
          <PlatformMcpServerFormFields form={form} keyEditable onChange={updateForm} />
        </SurfaceCard>

        {verificationResult ? <VerificationResultCard record={verificationResult} /> : null}
      </div>
    </>
  );
}
