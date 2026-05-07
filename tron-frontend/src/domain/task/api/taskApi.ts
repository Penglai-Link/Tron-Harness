export const taskApiEndpoints = {
  getTaskDetail: "GET /api/v1/tasks/{taskId}",
  listKanbanColumns: "GET /api/v1/tasks/kanban-columns",
  getTaskQuickReference: "GET /api/v1/tasks/quick-reference",
} as const;
