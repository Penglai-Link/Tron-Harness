export const common = {
  actions: {
    close: "关闭",
    logout: "退出登录",
    refresh: "刷新",
    signIn: "登录",
  },
  brand: {
    console: "主控制台",
    name: "Tron",
  },
  content: {
    none: "无",
  },
  responsesState: {
    label: "Responses 状态",
    stateless: "无状态",
    stateful: "有状态",
  },
  locale: {
    english: "English",
    simplifiedChinese: "简体中文",
    label: "语言",
    unavailable: "切换后立即生效，刷新后仍会保持当前语言。",
  },
  agentStatus: {
    disabled: "禁用",
    enabled: "启用",
    unavailable: "不可用",
  },
  pagination: {
    itemLabel: "记录",
    nextPage: "下一页",
    of: "共",
    page: "第",
    previousPage: "上一页",
    range: "显示 {{start}}-{{end}} / 共 {{total}} 条{{itemLabel}}",
    rows: "行",
    rowsPerPage: "每页行数",
    goToPage: "跳转到页",
    visible: "可见",
  },
  search: {
    ariaLabel: "搜索",
    clear: "清空搜索",
  },
  toolOperationKind: {
    command: "命令",
    query: "查询",
  },
  table: {
    empty: "当前筛选下没有记录。",
  },
} as const;
