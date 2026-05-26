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
  locale: {
    english: "English",
    label: "Language",
    unavailable: "Additional locales will be enabled after translation resources are approved.",
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
  table: {
    empty: "No records match the current filters.",
  },
} as const;
