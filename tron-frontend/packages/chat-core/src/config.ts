import type { ChatClientConfig, ChatEnvOptions } from "./types";

function envValue(
  env: Record<string, unknown>,
  prefix: string,
  key: string,
  fallbackPrefix: string | undefined,
) {
  return env[`${prefix}${key}`] ?? (fallbackPrefix ? env[`${fallbackPrefix}${key}`] : undefined);
}

function readBoolean(value: unknown, fallback = false) {
  if (value === undefined || value === null || value === "") return fallback;
  if (typeof value === "boolean") return value;
  return String(value).trim().toLowerCase() === "true";
}

function readNumber(value: unknown, fallback: number | undefined = undefined) {
  if (value === undefined || value === null || value === "") return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function readJson(value: unknown) {
  if (value === undefined || value === null || value === "") return undefined;
  try {
    return JSON.parse(String(value));
  } catch {
    return undefined;
  }
}

export function fromEnv(
  env: Record<string, unknown>,
  options: ChatEnvOptions = {},
): ChatClientConfig {
  const prefix = options.prefix ?? "VITE_TRON_CHAT_";
  const fallbackPrefix = options.fallbackPrefix ?? "VITE_GEWU_";

  return {
    accept: String(envValue(env, prefix, "ACCEPT", fallbackPrefix) ?? ""),
    agentCode: String(envValue(env, prefix, "AGENT_CODE", fallbackPrefix) ?? ""),
    appCode: String(envValue(env, prefix, "APP_CODE", fallbackPrefix) ?? ""),
    appKey: String(envValue(env, prefix, "APP_KEY", fallbackPrefix) ?? ""),
    baseUrl: String(
      envValue(env, prefix, "BASE_URL", fallbackPrefix)
      ?? envValue(env, prefix, "DOMAIN", fallbackPrefix)
      ?? "",
    ),
    defaultSessionCode: String(envValue(env, prefix, "DEFAULT_SESSION", fallbackPrefix) ?? ""),
    disabledInput: readBoolean(envValue(env, prefix, "DISABLED_INPUT", fallbackPrefix)),
    disabledToBottom: readBoolean(envValue(env, prefix, "DISABLED_TO_BOTTOM", fallbackPrefix)),
    floating: readBoolean(envValue(env, prefix, "FLOATING", fallbackPrefix), true),
    fullScreen: readBoolean(envValue(env, prefix, "FULL_SCREEN", fallbackPrefix)),
    fullScreenMinWidth: readNumber(envValue(env, prefix, "FULL_SCREEN_MIN_WIDTH", fallbackPrefix), 0),
    inputPlaceholder: String(
      envValue(env, prefix, "INPUT_PLACEHOLDER", fallbackPrefix)
      ?? "Enter to send, Shift+Enter for newline",
    ),
    maxFileSize: readNumber(envValue(env, prefix, "MAX_FILE_SIZE", fallbackPrefix), 20 * 1024 * 1024),
    maxLimitFile: readNumber(envValue(env, prefix, "MAX_LIMIT_FILE", fallbackPrefix), 10),
    openAppCode: String(
      envValue(env, prefix, "OPEN_APP_CODE", fallbackPrefix)
      ?? envValue(env, prefix, "APP_ID", fallbackPrefix)
      ?? "",
    ),
    productPrefix: String(envValue(env, prefix, "PRODUCT_PREFIX", fallbackPrefix) ?? ""),
    showAgui: readBoolean(envValue(env, prefix, "SHOW_AGUI", fallbackPrefix), true),
    showSessionList: readBoolean(envValue(env, prefix, "SHOW_SESSION_LIST", fallbackPrefix)),
    storageKey: String(envValue(env, prefix, "STORAGE_KEY", fallbackPrefix) ?? ""),
    taskCode: String(envValue(env, prefix, "TASK_CODE", fallbackPrefix) ?? ""),
    title: String(envValue(env, prefix, "TITLE", fallbackPrefix) ?? ""),
    tokenPrefix: String(envValue(env, prefix, "TOKEN_PREFIX", fallbackPrefix) ?? ""),
    userCode: String(
      envValue(env, prefix, "USER_CODE", fallbackPrefix)
      ?? envValue(env, prefix, "USER_ID", fallbackPrefix)
      ?? "",
    ),
    userId: String(envValue(env, prefix, "USER_ID", fallbackPrefix) ?? ""),
    variables: readJson(envValue(env, prefix, "VARIABLES", fallbackPrefix)),
    workspaceCode: String(envValue(env, prefix, "WORKSPACE_CODE", fallbackPrefix) ?? ""),
  };
}
