import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";
import { Field } from "@/components/forms";
import { SearchField } from "@/components/search-field";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface InternalAgentMcpServerSelectorProps {
  busy?: boolean;
  onSetServersChecked: (serverCodes: string[], checked: boolean) => void;
  onToggleServer: (serverCode: string, checked: boolean) => void;
  selectedServerCodes: string[];
  servers: WorkspaceMcpServerRecord[];
}

function normalizeSearchText(value: string) {
  return value.trim().toLowerCase();
}

function healthTone(status: WorkspaceMcpServerRecord["lastHealthStatus"]) {
  switch (status) {
    case "HEALTHY":
      return "success" as const;
    case "UNHEALTHY":
      return "warning" as const;
    default:
      return "info" as const;
  }
}

export function InternalAgentMcpServerSelector({
  busy = false,
  onSetServersChecked,
  onToggleServer,
  selectedServerCodes,
  servers,
}: InternalAgentMcpServerSelectorProps) {
  const { t } = useTranslation(["pages"]);
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const selectedServerSet = useMemo(() => new Set(selectedServerCodes), [selectedServerCodes]);

  const internalPageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.internal.mcpSelection.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(normalizeSearchText(search));
    }, 180);

    return () => window.clearTimeout(timer);
  }, [search]);

  const visibleServers = useMemo(() => {
    const normalizedSearch = debouncedSearch;
    return servers.filter((server) => {
      if (!normalizedSearch) {
        return true;
      }

      return [
        server.serverCode,
        server.name,
        server.description,
        server.lastHealthStatus,
        server.lastHealthMessage ?? "",
      ].join(" ").toLowerCase().includes(normalizedSearch);
    });
  }, [debouncedSearch, servers]);

  const assignableVisibleServers = visibleServers.filter((server) => server.effectiveEnabled);
  const allVisibleSelected = assignableVisibleServers.length > 0
    && assignableVisibleServers.every((server) => selectedServerSet.has(server.serverCode));
  const hasAnyVisibleSelected = assignableVisibleServers.some((server) => selectedServerSet.has(server.serverCode));

  return (
    <div className="mt-5 space-y-3">
      <Field label={internalPageT("title", "Selected MCP servers")}>
        <SearchField
          ariaLabel={internalPageT("searchAria", "Search MCP servers")}
          placeholder={internalPageT("searchPlaceholder", "Search MCP servers...")}
          value={search}
          onChange={setSearch}
        />
      </Field>

      <div className="flex flex-wrap items-center justify-between gap-[12px]">
        <p className="text-xs text-[var(--color-on-surface-variant)]">
          {internalPageT("showing", "Showing {{visible}} of {{total}} servers", {
            visible: visibleServers.length,
            total: servers.length,
          })}
        </p>
        <div className="flex flex-wrap gap-[12px]">
          <Button
            disabled={busy || allVisibleSelected || assignableVisibleServers.length === 0}
            size="sm"
            variant="secondary"
            onClick={() => {
              onSetServersChecked(assignableVisibleServers.map((server) => server.serverCode), true);
            }}
          >
            {internalPageT("selectVisible", "Select visible")}
          </Button>
          <Button
            disabled={busy || !hasAnyVisibleSelected}
            size="sm"
            variant="secondary"
            onClick={() => {
              onSetServersChecked(assignableVisibleServers.map((server) => server.serverCode), false);
            }}
          >
            {internalPageT("clearVisible", "Clear visible")}
          </Button>
        </div>
      </div>

      {visibleServers.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {internalPageT("empty", "No MCP servers match the search query.")}
        </p>
      ) : (
        <div className="grid gap-3 lg:grid-cols-2">
          {visibleServers.map((server) => {
            const disabled = !server.effectiveEnabled;
            const checked = selectedServerSet.has(server.serverCode);

            return (
              <label
                key={server.serverCode}
                className={`rounded-2xl border p-4 transition-colors ${checked
                    ? "border-[var(--color-primary)] bg-[var(--color-primary)]/8"
                    : "border-[var(--color-card-border)] bg-[var(--color-surface-container-lowest)]"
                  } ${disabled ? "opacity-70" : ""}`}
              >
                <div className="flex items-start gap-3">
                  <input
                    checked={checked}
                    className="mt-1 size-4 rounded border-[var(--color-outline-variant)] text-[var(--color-primary)] focus:ring-[var(--color-focus-ring)]"
                    disabled={busy || disabled}
                    type="checkbox"
                    onChange={(event) => {
                      onToggleServer(server.serverCode, event.currentTarget.checked);
                    }}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-[12px]">
                      <p className="text-sm font-semibold text-[var(--color-on-surface)]">
                        {server.name}
                      </p>
                      <Badge variant={healthTone(server.lastHealthStatus)}>
                        {server.lastHealthStatus}
                      </Badge>
                      {!server.platformEnabled ? (
                        <Badge variant="warning">
                          {internalPageT("platformDisabled", "Platform disabled")}
                        </Badge>
                      ) : null}
                      {server.workspaceOverrideDisabled ? (
                        <Badge variant="warning">
                          {internalPageT("workspaceDisabled", "Locally disabled")}
                        </Badge>
                      ) : null}
                    </div>
                    <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                      {server.serverCode}
                    </p>
                    <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
                      {server.description || internalPageT("noDescription", "No description available.")}
                    </p>
                    {server.lastHealthMessage ? (
                      <p className="mt-2 text-xs text-[var(--color-on-surface-variant)]">
                        {server.lastHealthMessage}
                      </p>
                    ) : null}
                    {disabled ? (
                      <p className="mt-2 text-xs text-[var(--color-warning)]">
                        {internalPageT(
                          "disabledHint",
                          "Only workspace-effective enabled servers can be assigned.",
                        )}
                      </p>
                    ) : null}
                  </div>
                </div>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}
