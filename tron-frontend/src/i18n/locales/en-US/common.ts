export const common = {
  actions: {
    close: "Close",
    logout: "Log out",
    refresh: "Refresh",
    signIn: "Sign In",
  },
  brand: {
    console: "Main Console",
    name: "Tron",
  },
  content: {
    none: "None",
  },
  responsesState: {
    label: "Responses State",
    stateless: "Stateless",
    stateful: "Stateful",
  },
  locale: {
    english: "English",
    simplifiedChinese: "Simplified Chinese",
    label: "Language",
    unavailable: "Changes apply immediately and remain active after reload.",
  },
  agentStatus: {
    disabled: "Disabled",
    enabled: "Enabled",
    unavailable: "Unavailable",
  },
  pagination: {
    itemLabel: "records",
    nextPage: "Next page",
    of: "of",
    page: "Page",
    previousPage: "Previous page",
    range: "{{start}}-{{end}} of {{total}} {{itemLabel}} visible",
    rows: "Rows",
    rowsPerPage: "Rows per page",
    goToPage: "Go to page",
    visible: "visible",
  },
  search: {
    ariaLabel: "Search",
    clear: "Clear search",
  },
  toolOperationKind: {
    command: "command",
    query: "query",
  },
  table: {
    empty: "No records match the current filters.",
  },
} as const;
