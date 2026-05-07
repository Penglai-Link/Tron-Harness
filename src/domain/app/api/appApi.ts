export const appApiEndpoints = {
  listApps: "GET /api/v1/apps",
  getImportDefaults: "GET /api/v1/apps/import-defaults?source={source}",
} as const;
