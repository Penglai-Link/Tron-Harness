import {
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  Pencil,
  Plus,
  RefreshCcw,
  Search,
  Trash2,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toastManager } from "@/components/ui/toast";
import {
  DataSourceCreateDialog,
  ServerAccountCreateDialog,
  ServerCreateDialog,
} from "@/domain/resource/components";
import { deleteResource } from "@/domain/resource/api";
import {
  useManagedResourceRecords,
  type ManagedResourceRecord,
} from "@/domain/resource/hooks";
import { PageHeader, useWorkspaceSpaces } from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type ResourceConnectionKind = "dataSource" | "server" | "serverAccount";

interface ResourceConnectionManagementPageProps {
  kind: ResourceConnectionKind;
}

const PAGE_SIZE = 15;

const KIND_CONFIG: Record<ResourceConnectionKind, {
  basePath: string;
  definitionCodes: readonly string[];
}> = {
  dataSource: {
    basePath: "/resource-management/data-sources",
    definitionCodes: [
      "MYSQL_CONNECTION",
      "POSTGRESQL_CONNECTION",
      "REDIS_CONNECTION",
      "KAFKA_CONNECTION",
    ],
  },
  server: {
    basePath: "/resource-management/servers",
    definitionCodes: ["SERVER_CONNECTION"],
  },
  serverAccount: {
    basePath: "/resource-management/server-accounts",
    definitionCodes: ["SERVER_ACCOUNT"],
  },
};

function firstEndpoint(attributes: Record<string, unknown>) {
  const values = Array.isArray(attributes.endpoints)
    ? attributes.endpoints
    : Array.isArray(attributes.brokers)
      ? attributes.brokers
      : [];
  const endpoint = values[0];
  if (!endpoint || typeof endpoint !== "object") return null;
  const { host, port } = endpoint as Record<string, unknown>;
  return typeof host === "string" && typeof port === "number" ? `${host}:${port}` : null;
}

function stringAttribute(record: ManagedResourceRecord, key: string) {
  const value = record.attributes[key];
  return typeof value === "string" ? value : "—";
}

export function ResourceConnectionManagementPage({ kind }: ResourceConnectionManagementPageProps) {
  const { t, i18n } = useTranslation("pages");
  const navigate = useNavigate();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = workspace?.workspaceCode ?? "";
  const workspaceName = workspace?.name ?? "";
  const config = KIND_CONFIG[kind];
  const [search, setSearch] = useState("");
  const [current, setCurrent] = useState(1);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ManagedResourceRecord | null>(null);
  const [deleting, setDeleting] = useState(false);
  const recordsQuery = useManagedResourceRecords({
    definitionCodes: config.definitionCodes,
    search,
    workspaceCode: workspaceCode || null,
    workspaceName,
  });
  const serversQuery = useManagedResourceRecords({
    definitionCodes: ["SERVER_CONNECTION"],
    search: "",
    workspaceCode: workspaceCode || null,
    workspaceName,
  });
  const pageT = (key: string, options?: Record<string, unknown>) => t(`resourceConnections.${key}`, options);
  const pageCount = Math.max(1, Math.ceil(recordsQuery.data.length / PAGE_SIZE));
  const pageRecords = useMemo(
    () => recordsQuery.data.slice((current - 1) * PAGE_SIZE, current * PAGE_SIZE),
    [current, recordsQuery.data],
  );

  function connectionLabel(record: ManagedResourceRecord) {
    if (kind === "server") {
      const host = stringAttribute(record, "host");
      const port = record.attributes.port;
      return `${host}:${typeof port === "number" ? port : "—"}`;
    }
    if (kind === "serverAccount") {
      return `${stringAttribute(record, "username")} @ ${stringAttribute(record, "serverCode")}`;
    }
    return firstEndpoint(record.attributes) ?? "—";
  }

  function secondaryLabel(record: ManagedResourceRecord) {
    if (kind === "serverAccount") {
      return pageT(`authentication.${stringAttribute(record, "authenticationType") === "PrivateKey" ? "privateKey" : "password"}`);
    }
    if (kind === "dataSource") {
      return record.definitionName;
    }
    return "SSH";
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      if (getServiceDataSource("resource") === "api") {
        await deleteResource({
          expectedActiveResourceVersion: deleteTarget.resourceVersion,
          expectedDraftResourceVersion: null,
          expectedDraftRevision: null,
          resourceCode: deleteTarget.resourceCode,
          scopeKind: "WORKSPACE",
          workspaceCode,
        });
      }
      toastManager.add({
        description: pageT("messages.deleted", { name: deleteTarget.name }),
        title: pageT("messages.deletedTitle"),
        type: "success",
      });
      setDeleteTarget(null);
      recordsQuery.refresh();
    } catch (cause) {
      toastManager.add({
        description: cause instanceof Error ? cause.message : pageT("messages.deleteFailed"),
        title: pageT("messages.deleteFailed"),
        type: "error",
      });
    } finally {
      setDeleting(false);
    }
  }

  const title = pageT(`${kind}.title`);

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col">
      <PageHeader
        title={title}
        actions={(
          <>
            <Button variant="outline" onClick={() => recordsQuery.refresh()}>
              <RefreshCcw aria-hidden="true" />
              {pageT("actions.refresh")}
            </Button>
            <Button disabled={!workspace} onClick={() => setCreateOpen(true)}>
              <Plus aria-hidden="true" />
              {pageT(`${kind}.createAction`)}
            </Button>
          </>
        )}
      />

      <div className="mb-4 flex max-w-md items-center gap-2">
        <div className="relative w-full">
          <Search aria-hidden="true" className="pointer-events-none absolute left-2.5 top-1/2 z-10 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            aria-label={pageT("search.label")}
            className="pl-8"
            placeholder={pageT(`${kind}.searchPlaceholder`)}
            type="search"
            value={search}
            onChange={(event) => {
              setSearch(event.target.value);
              setCurrent(1);
            }}
          />
        </div>
      </div>

      <div className="min-h-0 flex-1">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[34%]">{pageT("columns.name")}</TableHead>
              <TableHead>{pageT(kind === "serverAccount" ? "columns.accountBinding" : "columns.endpoint")}</TableHead>
              <TableHead>{pageT(kind === "dataSource" ? "columns.type" : kind === "serverAccount" ? "columns.authentication" : "columns.protocol")}</TableHead>
              <TableHead>{pageT("columns.source")}</TableHead>
              <TableHead>{pageT("columns.updatedAt")}</TableHead>
              <TableHead className="text-right">{pageT("columns.actions")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {recordsQuery.loading ? (
              <TableRow>
                <TableCell className="h-32 text-center" colSpan={6}><Spinner className="mx-auto" /></TableCell>
              </TableRow>
            ) : pageRecords.length ? pageRecords.map((record) => (
              <TableRow key={`${record.resourceCode}:${record.resourceVersion}`}>
                <TableCell>
                  <div className="min-w-0">
                    <p className="truncate font-medium">{record.name}</p>
                    <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">{record.resourceCode}</p>
                  </div>
                </TableCell>
                <TableCell className="font-mono text-xs">{connectionLabel(record)}</TableCell>
                <TableCell><Badge variant="outline">{secondaryLabel(record)}</Badge></TableCell>
                <TableCell>
                  <Badge variant={record.sourceKind === "LOCAL" ? "secondary" : "outline"}>
                    {pageT(`source.${record.sourceKind.toLowerCase()}`)}
                  </Badge>
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(record.updatedAt)}
                </TableCell>
                <TableCell>
                  <div className="flex justify-end gap-1">
                    <Button aria-label={pageT("actions.edit")} size="icon-sm" variant="ghost" onClick={() => navigate(`${config.basePath}/detail?resourceCode=${encodeURIComponent(record.resourceCode)}&resourceVersion=${encodeURIComponent(record.resourceVersion)}`)}>
                      <Pencil aria-hidden="true" />
                    </Button>
                    <Button aria-label={pageT("actions.delete")} size="icon-sm" variant="ghost" onClick={() => setDeleteTarget(record)}>
                      <Trash2 aria-hidden="true" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )) : (
              <TableRow>
                <TableCell className="h-32 text-center text-muted-foreground" colSpan={6}>
                  {workspace ? pageT(`${kind}.empty`) : pageT("workspaceRequired")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <div className="mt-4 flex items-center justify-between border-t pt-3 text-sm text-muted-foreground">
        <span>{pageT("pagination.total", { total: recordsQuery.data.length })}</span>
        <div className="flex items-center gap-2">
          <Button disabled={current <= 1} size="sm" variant="outline" onClick={() => setCurrent((value) => Math.max(1, value - 1))}>{pageT("pagination.previous")}</Button>
          <span>{current} / {pageCount}</span>
          <Button disabled={current >= pageCount} size="sm" variant="outline" onClick={() => setCurrent((value) => Math.min(pageCount, value + 1))}>{pageT("pagination.next")}</Button>
        </div>
      </div>

      {kind === "dataSource" ? (
        <DataSourceCreateDialog onCreated={recordsQuery.refresh} onOpenChange={setCreateOpen} open={createOpen} workspaceCode={workspaceCode} />
      ) : kind === "server" ? (
        <ServerCreateDialog onCreated={() => { recordsQuery.refresh(); serversQuery.refresh(); }} onOpenChange={setCreateOpen} open={createOpen} workspaceCode={workspaceCode} />
      ) : (
        <ServerAccountCreateDialog onCreated={recordsQuery.refresh} onOpenChange={setCreateOpen} open={createOpen} servers={serversQuery.data} workspaceCode={workspaceCode} />
      )}

      <Dialog onOpenChange={(open) => !open && setDeleteTarget(null)} open={Boolean(deleteTarget)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{pageT("delete.title", { name: deleteTarget?.name ?? "" })}</DialogTitle>
          </DialogHeader>
          <DialogFooter>
            <Button disabled={deleting} variant="outline" onClick={() => setDeleteTarget(null)}>{pageT("actions.cancel")}</Button>
            <Button disabled={deleting} variant="destructive" onClick={() => { void confirmDelete(); }}>
              {deleting ? <Spinner /> : null}
              {pageT("actions.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export function DataSourceManagementPage() {
  return <ResourceConnectionManagementPage kind="dataSource" />;
}

export function ServerManagementPage() {
  return <ResourceConnectionManagementPage kind="server" />;
}

export function ServerAccountManagementPage() {
  return <ResourceConnectionManagementPage kind="serverAccount" />;
}
