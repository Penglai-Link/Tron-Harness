import { useEffect, useState } from "react";
import { Plus, RefreshCcw, Trash2 } from "lucide-react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { deleteTaskFlow, fetchTaskFlows } from "@/domain/task/api";
import { deleteMockTaskFlow, listMockTaskFlows } from "@/domain/task/mock/data";
import {
  taskFlowDisplayName,
  taskFlowErrorMessage,
  taskFlowStatusDisplayName,
  type TaskFlow,
} from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { PageHeader, useTablePagination } from "@/shared";

export function TaskFlowsPage() {
  const { t } = useTranslation(["pages"]);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const [flows, setFlows] = useState<TaskFlow[]>([]);
  const [loading, setLoading] = useState(false);
  const [flowToDelete, setFlowToDelete] = useState<TaskFlow | null>(null);
  const [deleting, setDeleting] = useState(false);
  const pagination = useTablePagination(flows);
  const flowT = (key: string, defaultValue: string) =>
    t(`pages:tasks.taskFlows.${key}`, defaultValue);

  async function refresh() {
    setLoading(true);
    try {
      setFlows(dataSource === "api" ? await fetchTaskFlows() : listMockTaskFlows());
    } catch (error) {
      toast.error(taskFlowErrorMessage(error, flowT, "errors.load", "Task flows could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { void refresh(); }, [dataSource]);

  async function confirmDelete() {
    if (!flowToDelete) return;
    setDeleting(true);
    try {
      if (dataSource === "api") await deleteTaskFlow(flowToDelete.flowCode);
      else deleteMockTaskFlow(flowToDelete.flowCode);
      setFlowToDelete(null);
      toast.success(flowT("messages.deleted", "Task flow deleted"));
      await refresh();
    } catch (error) {
      toast.error(taskFlowErrorMessage(error, flowT, "errors.delete", "Task flow could not be deleted"));
    } finally {
      setDeleting(false);
    }
  }

  return (
    <>
      <PageHeader
        title={flowT("title", "Task Flows")}
        actions={(
          <>
            <Button disabled={loading} variant="secondary" onClick={() => void refresh()}><RefreshCcw size={16} />{flowT("actions.refresh", "Refresh")}</Button>
            <Button onClick={() => navigate("/settings/task-flows/new")}><Plus size={16} />{flowT("actions.new", "New")}</Button>
          </>
        )}
      />
      <DataTable<TaskFlow>
        columns={[
          { id: "flowCode", header: flowT("columns.flowCode", "Code"), render: (flow) => <span className="font-mono text-sm">{flow.flowCode}</span> },
          { id: "name", header: flowT("columns.name", "Name"), render: (flow) => <span className="font-medium">{taskFlowDisplayName(flow, flowT)}</span> },
          { id: "statuses", header: flowT("columns.statuses", "Statuses"), render: (flow) => <span className="text-sm text-[var(--color-on-surface-variant)]">{flow.statuses.map((status) => taskFlowStatusDisplayName(status, flowT, flow.flowCode)).join(" · ")}</span> },
          {
            id: "actions",
            header: flowT("columns.actions", "Actions"),
            align: "right",
            render: (flow) => (
              <Button
                aria-label={t("pages:tasks.taskFlows.aria.deleteFlow", "Delete {{name}}", { name: taskFlowDisplayName(flow, flowT) })}
                disabled={flow.flowCode === "DEFAULT"}
                size="icon"
                variant="ghost"
                onClick={(event) => { event.stopPropagation(); setFlowToDelete(flow); }}
              ><Trash2 size={16} /></Button>
            ),
          },
        ]}
        emptyState={flowT("empty", "No task flows yet.")}
        pagination={<TablePagination itemLabel={flowT("pagination.itemLabel", "task flows")} {...pagination.paginationProps} />}
        rowKey={(flow) => flow.flowCode}
        rows={pagination.visibleItems}
        onRowClick={(flow) => navigate(`/settings/task-flows/edit?flowCode=${encodeURIComponent(flow.flowCode)}`)}
      />
      <ConfirmDialog
        cancelText={flowT("actions.cancel", "Cancel")}
        confirmText={flowT("actions.delete", "Delete")}
        description={flowT("confirm.deleteDescription", "Only task flows not used by a workspace or task can be deleted.")}
        onCancel={() => setFlowToDelete(null)}
        onConfirm={() => void confirmDelete()}
        open={Boolean(flowToDelete)}
        pending={deleting}
        title={flowT("confirm.deleteTitle", "Delete task flow")}
      />
    </>
  );
}
