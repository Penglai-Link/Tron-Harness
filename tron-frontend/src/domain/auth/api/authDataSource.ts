export type AuthDataSource = "mock" | "api";

function normalizeAuthDataSource(value: unknown): AuthDataSource | undefined {
  if (typeof value !== "string") return undefined;
  const normalized = value.trim().toLowerCase();
  return normalized === "api" || normalized === "mock" ? normalized : undefined;
}

function getEnvValue(key: string) {
  return (import.meta.env as Record<string, string | undefined>)[key];
}

export function getAuthDataSource(): AuthDataSource {
  return normalizeAuthDataSource(getEnvValue("VITE_TRON_AUTH_DATA_SOURCE"))
    ?? normalizeAuthDataSource(getEnvValue("VITE_TRON_DATA_SOURCE"))
    ?? "mock";
}
