import {
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { FileText } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  PageHeader,
  SelectInput,
  TablePagination,
  TextInput,
  useTablePagination,
} from "@/shared";

import { useAuditLogList } from "@/domain/admin/hooks";
import type { AuditLogRecord } from "@/domain/admin/model";
import {
  adminAuditStatusOptions,
  getStatusTone,
} from "@/domain/admin/constants";
import { SelectItem } from "@/components/ui/select";

export function AuditLogsPage() {
  const { t } = useTranslation(["pages"]);
  const rows = useAuditLogList();
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [notice, setNotice] = useState("");
  const filteredRows = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return rows.filter((row) => (status === "all" || row.status === status) && (!normalized || Object.values(row).join(" ").toLowerCase().includes(normalized)));
  }, [query, rows, status]);
  const auditPagination = useTablePagination(filteredRows);

  return (
    <>
      <PageHeader
        title={t("pages:admin.audit.title", "Audit Logs")}
        actions={
          <Button variant="secondary" onClick={() => setNotice(t("pages:admin.audit.exported", "{{count}} audit rows exported", { count: filteredRows.length }))}>
            <FileText size={15} />
            {t("pages:admin.audit.export", "Export")}
          </Button>
        }
      />
      <p className="sr-only" aria-live="polite">{notice}</p>
      <DataTable<AuditLogRecord>
        title={t("pages:admin.audit.title", "Audit Logs")}
        rows={auditPagination.visibleItems}
        rowKey={(row) => row.id}
        toolbar={
          <>
            <TextInput className="h-8 w-60" placeholder={t("pages:admin.audit.search", "Search audit logs...")} value={query} onChange={(event) => setQuery(event.target.value)} />
            <SelectInput className="h-8 w-36" value={status} onValueChange={(event) => setStatus(event!)}>
              {adminAuditStatusOptions.map((option) => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}
            </SelectInput>
          </>
        }
        pagination={
          <TablePagination itemLabel={t("pages:admin.audit.entries", "entries")} {...auditPagination.paginationProps} />
        }
        columns={[
          { id: "timestamp", header: t("pages:admin.audit.columns.timestamp", "Timestamp"), render: (row) => <span className="font-mono text-[13px]">{row.timestamp}</span> },
          { id: "actor", header: t("pages:admin.audit.columns.actor", "Actor"), render: (row) => row.actor },
          { id: "action", header: t("pages:admin.audit.columns.action", "Action"), render: (row) => row.action },
          { id: "resource", header: t("pages:admin.audit.columns.resource", "Resource ID"), render: (row) => <span className="font-mono text-[13px]">{row.resourceId}</span> },
          { id: "status", header: t("pages:admin.audit.columns.status", "Status"), render: (row) => <Badge variant={getStatusTone(row.status)}>{row.status}</Badge> },
          { id: "ip", header: t("pages:admin.audit.columns.ip", "IP Address"), align: "right", render: (row) => <span className="font-mono text-[13px]">{row.ipAddress}</span> },
        ]}
      />
    </>
  );
}
