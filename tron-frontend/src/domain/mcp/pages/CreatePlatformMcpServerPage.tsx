import {
  useState,
} from "react";
import { useNavigate } from "react-router";
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
  Badge,
  Button,
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useToast,
} from "@/shared";

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
  const { formatDateTime } = useLocaleFormatters();

  return (
    <SurfaceCard className="space-y-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Verification result</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            Draft configuration verified against the platform MCP runtime.
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
  );
}

export function CreatePlatformMcpServerPage() {
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
      toast.success(`${verified.name} verified`);
    } catch (error) {
      setVerificationResult(null);
      toast.error(error instanceof Error ? error.message : "Verify failed");
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
      toast.success(`${saved.name} created`);
      navigate(`/settings/mcp-servers/detail?serverCode=${encodeURIComponent(saved.serverCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform MCP server could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: "MCP Servers", href: "/settings/mcp-servers" },
          { label: "New" },
        ]}
        title="Connect MCP server"
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/mcp-servers")}>Cancel</Button>
            <Button
              variant="secondary"
              disabled={submitting || verifying || Boolean(validationError)}
              aria-busy={verifying}
              onClick={() => void verifyDraft()}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {verifying ? "Verifying..." : "Verify"}
            </Button>
            <Button
              disabled={submitting || verifying || Boolean(validationError)}
              aria-busy={submitting}
              onClick={() => void saveServer()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
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
