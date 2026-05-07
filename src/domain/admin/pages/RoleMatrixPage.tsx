import { useState } from "react";
import {
  Filter,
  Plus,
  Search,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SegmentedTabs,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import {
  usePermissionDomains,
  useRoles
} from "@/domain/admin/hooks";
import { cn } from "@/shared/lib/utils";
import {
  adminManagementTabs,
  adminPermissionCodes,
  adminPermissionLegend,
} from "@/domain/admin/constants";

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

export function RoleMatrixPage() {
  const roles = useRoles();
  const domains = usePermissionDomains();
  const [activeTab, setActiveTab] = useState("roles");
  const [query, setQuery] = useState("");
  const [overrides, setOverrides] = useState<Record<string, string[]>>({});
  const [notice, setNotice] = useState("Role matrix ready.");
  const filteredRoles = roles.filter((role) => role.name.toLowerCase().includes(query.toLowerCase()));

  function togglePermission(roleId: string, domain: string, code: string) {
    const key = `${roleId}:${domain}`;
    const base = roles.find((role) => role.id === roleId)?.permissions[domain] ?? [];
    const current = overrides[key] ?? base;
    setOverrides((values) => ({ ...values, [key]: current.includes(code) ? current.filter((item) => item !== code) : [...current, code] }));
  }

  return (
    <TronShell activeItem="Roles" searchPlaceholder="Search users or roles...">
      <PageHeader
        title="Users & Roles"
        status="Permission grid"
        actions={<Button onClick={() => setNotice("Custom role editor opened")}><Plus size={16} aria-hidden="true" />Add Custom Role</Button>}
      />
      <SurfaceCard className="mb-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <SegmentedTabs value={activeTab} onValueChange={setActiveTab} items={adminManagementTabs} />
          <div className="flex items-center gap-2">
            <SearchField placeholder="Search roles..." value={query} onChange={setQuery} />
            <Button size="sm" variant="secondary" onClick={() => setNotice(`Filter drawer opened for ${filteredRoles.length} roles`)}><Filter size={14} aria-hidden="true" />Filter</Button>
          </div>
        </div>
      </SurfaceCard>
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="overflow-hidden p-0">
        <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-5">
          <h2 className="text-xl font-semibold">Platform Permissions</h2>
          <Badge tone="accent">{activeTab === "roles" ? "Role Matrix" : "Related tab selected"}</Badge>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[980px] border-collapse text-left text-sm">
            <thead>
              <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]">
                <th className="sticky left-0 z-10 w-48 border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[11px] uppercase text-[var(--color-on-surface-variant)]">Role Name</th>
                {domains.map((domain) => <th className="border-r border-[var(--color-outline-variant)] p-4 text-center text-[11px] uppercase text-[var(--color-on-surface-variant)]" key={domain}>{domain}</th>)}
              </tr>
            </thead>
            <tbody>
              {filteredRoles.map((role) => (
                <tr className="border-b border-[var(--color-outline-variant)] hover:bg-[var(--color-hover-surface)]" key={role.id}>
                  <td className="sticky left-0 border-r border-[var(--color-outline-variant)] bg-white p-4 font-medium">
                    {role.name}
                    <div className="mt-1 text-[11px] text-[var(--color-on-surface-variant)]">{role.description}</div>
                  </td>
                  {domains.map((domain) => {
                    const permissions = overrides[`${role.id}:${domain}`] ?? role.permissions[domain] ?? [];
                    return (
                      <td className="border-r border-[var(--color-outline-variant)] p-4" key={domain}>
                        <div className="flex justify-center gap-1">
                          {adminPermissionCodes.map((code) => {
                            const active = permissions.includes(code);
                            return (
                              <button className={cn("flex size-6 items-center justify-center rounded text-[10px] font-semibold", active ? "bg-[var(--color-primary)] text-[var(--color-on-primary)]" : "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)] opacity-60")} type="button" key={code} title={code} onClick={() => togglePermission(role.id, domain, code)}>
                                {code}
                              </button>
                            );
                          })}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="flex flex-wrap items-center gap-4 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[13px] text-[var(--color-on-surface-variant)]">
          {adminPermissionLegend.map((item) => <span key={item}>{item}</span>)}
        </div>
      </SurfaceCard>
    </TronShell>
  );
}
