import {
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import type {
  ManagedOpenAppCapabilityGrant,
  ManagedOpenAppDetail,
  ManagedOpenAppSecretReveal,
  OpenApiCapabilityCatalogGroup,
  OpenApiCapabilityCatalogOperation,
  OpenAppStatus,
} from "@/domain/open-app/model";
import {
  Field,
  MetricCard,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useLocaleFormatters,
} from "@/shared";
import {
  ChevronDown,
  ChevronRight,
  KeyRound,
  ShieldCheck,
} from "lucide-react";

export interface MetadataDraft {
  description: string;
  name: string;
  status: OpenAppStatus;
}

export const emptyMetadataDraft: MetadataDraft = {
  description: "",
  name: "",
  status: "Enabled",
};

export function useOpenAppPageT() {
  const { t } = useTranslation(["pages"]);

  return (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsOpenApi.${key}`, { defaultValue, ...options })
  );
}

export function statusTone(status: OpenAppStatus) {
  switch (status) {
    case "Enabled":
      return "success";
    case "Disabled":
      return "warning";
    case "Archived":
      return "neutral";
  }
}

export function formatOpenAppStatusLabel(
  pageT: (key: string, defaultValue: string, options?: Record<string, unknown>) => string,
  status: OpenAppStatus,
) {
  switch (status) {
    case "Enabled":
      return pageT("status.enabled", "Enabled");
    case "Disabled":
      return pageT("status.disabled", "Disabled");
    case "Archived":
      return pageT("status.archived", "Archived");
  }
}

export function capabilityGrantKey(capabilityCode: string, operationCode: string) {
  return `${capabilityCode}:${operationCode}`;
}

export function normalizeUserCodesInput(input: string) {
  return Array.from(new Set(
    input
      .split(/[\n,]+/)
      .map((value) => value.trim())
      .filter(Boolean),
  )).sort();
}

function formatCapabilityDisplayName(capabilityName: string) {
  const trimmed = capabilityName.replace(/\s+OpenAPI$/i, "").trim();
  return trimmed || capabilityName;
}

function checkboxDomId(...parts: string[]) {
  return parts.join("-").replace(/[^a-zA-Z0-9_-]/g, "-");
}

function capabilityOperationKeys(group: OpenApiCapabilityCatalogGroup) {
  return group.operations.map((operation) => capabilityGrantKey(group.capabilityCode, operation.operationCode));
}

function retainMatchingValues(current: string[], allowedValues: Set<string>) {
  const next = current.filter((value) => allowedValues.has(value));
  if (next.length === current.length && next.every((value, index) => value === current[index])) {
    return current;
  }
  return next;
}

function IndeterminateCheckbox({
  ariaChecked,
  ariaLabel,
  checked,
  className,
  disabled,
  id,
  indeterminate,
  onChange,
}: {
  ariaChecked?: "false" | "mixed" | "true";
  ariaLabel?: string;
  checked: boolean;
  className?: string;
  disabled?: boolean;
  id?: string;
  indeterminate: boolean;
  onChange: () => void;
}) {
  const ref = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!ref.current) return;
    ref.current.indeterminate = indeterminate;
  }, [indeterminate]);

  return (
    <input
      ref={ref}
      id={id}
      type="checkbox"
      aria-checked={ariaChecked}
      aria-label={ariaLabel}
      checked={checked}
      className={className}
      disabled={disabled}
      onChange={onChange}
    />
  );
}

function formatGrantedCapabilityGroups(
  grants: ManagedOpenAppCapabilityGrant[],
  catalog: OpenApiCapabilityCatalogGroup[],
) {
  const grouped = new Map<string, Array<{
    description: string;
    operationCode: string;
    operationName: string;
    routes: string[];
  }>>();

  for (const grant of grants) {
    const catalogGroup = catalog.find((item) => item.capabilityCode === grant.capabilityCode);
    const operation = catalogGroup?.operations.find((item) => item.operationCode === grant.operationCode);
    const entry = {
      description: operation?.description ?? "",
      operationCode: grant.operationCode,
      operationName: operation?.operationName ?? grant.operationCode,
      routes: operation?.routes ?? [],
    };
    const current = grouped.get(grant.capabilityCode) ?? [];
    current.push(entry);
    grouped.set(grant.capabilityCode, current);
  }

  return Array.from(grouped.entries()).map(([capabilityCode, operations]) => {
    const catalogGroup = catalog.find((item) => item.capabilityCode === capabilityCode);
    return {
      capabilityCode,
      capabilityDescription: catalogGroup?.description ?? "",
      capabilityName: catalogGroup?.capabilityName ?? capabilityCode,
      operations: operations.sort((left, right) => left.operationName.localeCompare(right.operationName)),
    };
  });
}

function ReadValue({ children }: { children: string }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children}
    </div>
  );
}

export function OpenAppMetrics({
  activeCount,
  catalogCount,
  recordCount,
}: {
  activeCount: number;
  catalogCount: number;
  recordCount: number;
}) {
  const pageT = useOpenAppPageT();

  return (
    <div className="grid gap-4 md:grid-cols-3">
      <MetricCard
        label={pageT("overview.openAppsLabel", "Open Apps")}
        value={recordCount.toString()}
        icon={<ShieldCheck size={18} aria-hidden="true" />}
      />
      <MetricCard
        label={pageT("overview.enabledLabel", "Enabled")}
        value={activeCount.toString()}
        icon={<KeyRound size={18} aria-hidden="true" />}
      />
      <MetricCard
        label={pageT("overview.capabilityGroupsLabel", "Capability Groups")}
        value={catalogCount.toString()}
        icon={<ShieldCheck size={18} aria-hidden="true" />}
      />
    </div>
  );
}

export function OpenAppSecretRevealCard({
  copyPending,
  onCopy,
  onDismiss,
  secretReveal,
}: {
  copyPending: boolean;
  onCopy: () => void;
  onDismiss?: () => void;
  secretReveal: ManagedOpenAppSecretReveal;
}) {
  const pageT = useOpenAppPageT();

  return (
    <SurfaceCard className="border-[var(--color-warning-container)] bg-[linear-gradient(135deg,rgba(255,237,196,0.65),rgba(255,255,255,0.9))]">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="inline-flex items-center gap-2 rounded-full bg-[var(--color-warning-container)] px-3 py-1 text-[12px] font-semibold text-[var(--color-on-warning-container)]">
            <KeyRound size={14} aria-hidden="true" />
            {pageT("reveal.badge", "One-time secret reveal")}
          </div>
          <h2 className="mt-3 text-lg font-semibold text-[var(--color-on-surface)]">
            {pageT("reveal.title", "Store this app key now")}
          </h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("reveal.description", "This cleartext key is only shown after create or rotate. Later views expose fingerprint only.")}
          </p>
        </div>
        {onDismiss ? (
          <button
            type="button"
            className="inline-flex min-h-10 items-center justify-center rounded-full px-4 text-sm font-medium text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-surface-container)]"
            onClick={onDismiss}
          >
            {pageT("actions.dismiss", "Dismiss")}
          </button>
        ) : null}
      </div>
      <div className="mt-4 grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.4fr)]">
        <div className="rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-4 py-3">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
            {pageT("fields.openAppCode", "Open App Code")}
          </p>
          <p className="mt-2 break-all font-mono text-sm text-[var(--color-on-surface)]">{secretReveal.openAppCode}</p>
        </div>
        <div className="rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-4 py-3">
          <div className="flex items-center justify-between gap-3">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              {pageT("fields.appKey", "App Key")}
            </p>
            <button
              type="button"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[var(--color-outline-variant)] px-3 text-sm font-medium text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-surface-container)] disabled:cursor-not-allowed disabled:opacity-70"
              disabled={copyPending}
              onClick={onCopy}
            >
              {copyPending ? pageT("actions.copying", "Copying...") : pageT("actions.copyKey", "Copy key")}
            </button>
          </div>
          <p className="mt-2 break-all font-mono text-sm text-[var(--color-on-surface)]">{secretReveal.appKey}</p>
          <p className="mt-3 text-[12px] text-[var(--color-on-surface-variant)]">
            {pageT("reveal.fingerprint", "Fingerprint: {{value}}", { value: secretReveal.appKeyFingerprint })}
          </p>
        </div>
      </div>
    </SurfaceCard>
  );
}

export function OpenAppMetadataFormFields({
  draft,
  onChange,
  showStatus = true,
  statusDisabled = false,
}: {
  draft: MetadataDraft;
  onChange: (next: MetadataDraft) => void;
  showStatus?: boolean;
  statusDisabled?: boolean;
}) {
  const pageT = useOpenAppPageT();

  return (
    <>
      <div className={`grid gap-4 ${showStatus ? "md:grid-cols-2" : ""}`}>
        <Field label={pageT("fields.name", "Name")}>
          <TextInput
            aria-label={pageT("fields.name", "Name")}
            placeholder={pageT("placeholders.name", "Customer Support Bridge")}
            value={draft.name}
            onChange={(event) => onChange({ ...draft, name: event.target.value })}
          />
        </Field>
        {showStatus ? (
          <Field label={pageT("fields.status", "Status")}>
            <SelectInput
              aria-label={pageT("fields.status", "Status")}
              disabled={statusDisabled}
              value={draft.status}
              onChange={(event) => (
                onChange({
                  ...draft,
                  status: event.target.value as OpenAppStatus,
                })
              )}
            >
              <option value="Enabled">{pageT("status.enabled", "Enabled")}</option>
              <option value="Disabled">{pageT("status.disabled", "Disabled")}</option>
              <option value="Archived">{pageT("status.archived", "Archived")}</option>
            </SelectInput>
          </Field>
        ) : null}
      </div>

      <Field label={pageT("fields.description", "Description")}>
        <TextArea
          aria-label={pageT("fields.description", "Description")}
          placeholder={pageT("placeholders.description", "What business system is this open-app allowed to represent?")}
          value={draft.description}
          onChange={(event) => onChange({ ...draft, description: event.target.value })}
        />
      </Field>
    </>
  );
}

export function OpenAppMetadataSummarySection({
  detail,
}: {
  detail: ManagedOpenAppDetail;
}) {
  const { formatDateTime } = useLocaleFormatters();
  const pageT = useOpenAppPageT();

  function formatTimestamp(value: string | null) {
    if (!value) {
      return pageT("values.never", "Never");
    }
    return formatDateTime(value, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  return (
    <SurfaceCard className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">
          {pageT("metadata.title", "Open App Metadata")}
        </h2>
      </div>

      <div className="grid gap-x-8 gap-y-6 md:grid-cols-2">
        <Field label={pageT("fields.name", "Name")}>
          <ReadValue>{detail.name}</ReadValue>
        </Field>
        <Field label={pageT("fields.openAppCode", "Open App Code")}>
          <ReadValue>{detail.openAppCode}</ReadValue>
        </Field>
        <Field label={pageT("fields.status", "Status")}>
          <ReadValue>{formatOpenAppStatusLabel(pageT, detail.status)}</ReadValue>
        </Field>
        <Field label={pageT("fields.currentFingerprint", "Current Fingerprint")}>
          <ReadValue>{detail.appKeyFingerprint || pageT("values.notAvailable", "N/A")}</ReadValue>
        </Field>
        <div className="md:col-span-2">
          <Field label={pageT("fields.description", "Description")}>
            <ReadValue>{detail.description || pageT("metadata.noDescription", "No description")}</ReadValue>
          </Field>
        </div>
        <Field label={pageT("fields.lastRotatedAt", "Last Rotated")}>
          <ReadValue>{formatTimestamp(detail.lastRotatedAt)}</ReadValue>
        </Field>
        <Field label={pageT("fields.createdAt", "Created")}>
          <ReadValue>{formatTimestamp(detail.createdAt)}</ReadValue>
        </Field>
        <Field label={pageT("fields.updatedAt", "Updated")}>
          <ReadValue>{formatTimestamp(detail.updatedAt)}</ReadValue>
        </Field>
      </div>
    </SurfaceCard>
  );
}

export function OpenAppIdentitySummarySection({
  detail,
}: {
  detail: ManagedOpenAppDetail;
}) {
  const { formatDateTime } = useLocaleFormatters();
  const pageT = useOpenAppPageT();

  function formatTimestamp(value: string | null) {
    if (!value) {
      return pageT("values.never", "Never");
    }
    return formatDateTime(value, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  return (
    <SurfaceCard className="space-y-6">
      <div className="grid gap-6 md:grid-cols-2">
        <Field label={pageT("fields.openAppCode", "Open App Code")}>
          <ReadValue>{detail.openAppCode}</ReadValue>
        </Field>
        <Field label={pageT("fields.currentFingerprint", "Current Fingerprint")}>
          <ReadValue>{detail.appKeyFingerprint || pageT("values.notAvailable", "N/A")}</ReadValue>
        </Field>
        <Field label={pageT("fields.lastRotatedAt", "Last Rotated")}>
          <ReadValue>{formatTimestamp(detail.lastRotatedAt)}</ReadValue>
        </Field>
        <Field label={pageT("fields.createdAt", "Created")}>
          <ReadValue>{formatTimestamp(detail.createdAt)}</ReadValue>
        </Field>
        <Field label={pageT("fields.updatedAt", "Updated")}>
          <ReadValue>{formatTimestamp(detail.updatedAt)}</ReadValue>
        </Field>
      </div>
    </SurfaceCard>
  );
}

export function OpenAppUserGrantEditorSection({
  disabled,
  footer,
  onChange,
  value,
}: {
  disabled: boolean;
  footer?: ReactNode;
  onChange: (next: string) => void;
  value: string;
}) {
  const pageT = useOpenAppPageT();

  return (
    <SurfaceCard className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-[var(--color-on-surface)]">
          {pageT("userGrants.title", "User Grants")}
        </h3>
      </div>

      <Field
        hint={pageT("userGrants.hint", "Signed requests still send X-Tron-User-Code explicitly. Signature success only proves the caller; this list proves who the open-app may impersonate.")}
        label={pageT("fields.allowedUserCodes", "Allowed User Codes")}
      >
        <TextArea
          aria-label={pageT("fields.allowedUserCodes", "Allowed User Codes")}
          disabled={disabled}
          placeholder={"USR-0001\nUSR-0002"}
          value={value}
          onChange={(event) => onChange(event.target.value)}
        />
      </Field>
      {footer}
    </SurfaceCard>
  );
}

export function OpenAppCapabilityGrantEditorSection({
  catalog,
  disabled,
  footer,
  onSetCapabilitySelection,
  onToggle,
  selectedGrantKeys,
}: {
  catalog: OpenApiCapabilityCatalogGroup[];
  disabled: boolean;
  footer?: ReactNode;
  onSetCapabilitySelection: (capabilityCode: string, operationCodes: string[], selected: boolean) => void;
  onToggle: (capabilityCode: string, operationCode: string) => void;
  selectedGrantKeys: string[];
}) {
  const pageT = useOpenAppPageT();
  const selectedGrantKeySet = new Set(selectedGrantKeys);
  const [expandedCapabilityCodes, setExpandedCapabilityCodes] = useState<string[]>([]);
  const [expandedOperationKeys, setExpandedOperationKeys] = useState<string[]>([]);

  useEffect(() => {
    const validCapabilityCodes = new Set(catalog.map((group) => group.capabilityCode));
    const validOperationKeys = new Set(
      catalog.flatMap((group) => group.operations.map((operation) => capabilityGrantKey(group.capabilityCode, operation.operationCode))),
    );
    setExpandedCapabilityCodes((current) => retainMatchingValues(current, validCapabilityCodes));
    setExpandedOperationKeys((current) => retainMatchingValues(current, validOperationKeys));
  }, [catalog]);

  function toggleExpandedCapability(capabilityCode: string) {
    setExpandedCapabilityCodes((current) => (
      current.includes(capabilityCode)
        ? current.filter((code) => code !== capabilityCode)
        : [...current, capabilityCode]
    ));
  }

  function toggleExpandedOperation(operationKey: string) {
    setExpandedOperationKeys((current) => (
      current.includes(operationKey)
        ? current.filter((key) => key !== operationKey)
        : [...current, operationKey]
    ));
  }

  return (
    <SurfaceCard className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-[var(--color-on-surface)]">
          {pageT("capabilityGrants.title", "OpenAPI Grants")}
        </h3>
      </div>

      <div className="space-y-4">
        {catalog.map((group) => (
          <CapabilityGrantGroupCard
            key={group.capabilityCode}
            disabled={disabled}
            expanded={expandedCapabilityCodes.includes(group.capabilityCode)}
            group={group}
            pageT={pageT}
            selectedGrantKeySet={selectedGrantKeySet}
            expandedOperationKeys={expandedOperationKeys}
            onSetCapabilitySelection={onSetCapabilitySelection}
            onToggle={onToggle}
            onToggleExpandedCapability={toggleExpandedCapability}
            onToggleExpandedOperation={toggleExpandedOperation}
          />
        ))}
      </div>
      {footer}
    </SurfaceCard>
  );
}

function CapabilityGrantGroupCard({
  disabled,
  expanded,
  expandedOperationKeys,
  group,
  onSetCapabilitySelection,
  onToggle,
  onToggleExpandedCapability,
  onToggleExpandedOperation,
  pageT,
  selectedGrantKeySet,
}: {
  disabled: boolean;
  expanded: boolean;
  expandedOperationKeys: string[];
  group: OpenApiCapabilityCatalogGroup;
  onSetCapabilitySelection: (capabilityCode: string, operationCodes: string[], selected: boolean) => void;
  onToggle: (capabilityCode: string, operationCode: string) => void;
  onToggleExpandedCapability: (capabilityCode: string) => void;
  onToggleExpandedOperation: (operationKey: string) => void;
  pageT: (key: string, defaultValue: string, options?: Record<string, unknown>) => string;
  selectedGrantKeySet: Set<string>;
}) {
  const capabilityDisplayName = formatCapabilityDisplayName(group.capabilityName);
  const operationKeys = capabilityOperationKeys(group);
  const selectedCount = operationKeys.filter((key) => selectedGrantKeySet.has(key)).length;
  const allSelected = operationKeys.length > 0 && selectedCount === operationKeys.length;
  const indeterminate = selectedCount > 0 && selectedCount < operationKeys.length;
  const checkboxId = checkboxDomId("open-app-capability", group.capabilityCode);

  return (
    <div className="rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-lowest)]">
      <div className="flex items-center justify-between gap-3 px-4 py-3">
        <IndeterminateCheckbox
          id={checkboxId}
          ariaChecked={indeterminate ? "mixed" : allSelected ? "true" : "false"}
          ariaLabel={capabilityDisplayName}
          checked={allSelected}
          className="size-4 accent-[var(--color-primary)]"
          disabled={disabled}
          indeterminate={indeterminate}
          onChange={() => onSetCapabilitySelection(
            group.capabilityCode,
            group.operations.map((operation) => operation.operationCode),
            !allSelected,
          )}
        />
        <button
          type="button"
          className="flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl px-2 py-1.5 text-left transition-colors hover:bg-[var(--color-surface-container)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-inset"
          aria-expanded={expanded}
          aria-label={expanded
            ? pageT("capabilityGrants.collapseCapability", "Collapse {{name}}", { name: capabilityDisplayName })
            : pageT("capabilityGrants.expandCapability", "Expand {{name}}", { name: capabilityDisplayName })}
          onClick={() => onToggleExpandedCapability(group.capabilityCode)}
        >
          <span className="min-w-0 text-sm font-semibold text-[var(--color-on-surface)]">
            {capabilityDisplayName}
          </span>
          {expanded ? <ChevronDown size={16} aria-hidden="true" /> : <ChevronRight size={16} aria-hidden="true" />}
        </button>
      </div>

      {expanded ? (
        <div className="border-t border-[var(--color-card-border)] px-3 py-3">
          <div className="space-y-2">
            {group.operations.map((operation) => (
              <CapabilityGrantOperationCard
                key={capabilityGrantKey(group.capabilityCode, operation.operationCode)}
                capabilityCode={group.capabilityCode}
                disabled={disabled}
                expanded={expandedOperationKeys.includes(capabilityGrantKey(group.capabilityCode, operation.operationCode))}
                operation={operation}
                pageT={pageT}
                selected={selectedGrantKeySet.has(capabilityGrantKey(group.capabilityCode, operation.operationCode))}
                onToggle={onToggle}
                onToggleExpandedOperation={onToggleExpandedOperation}
              />
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function CapabilityGrantOperationCard({
  capabilityCode,
  disabled,
  expanded,
  operation,
  onToggle,
  onToggleExpandedOperation,
  pageT,
  selected,
}: {
  capabilityCode: string;
  disabled: boolean;
  expanded: boolean;
  operation: OpenApiCapabilityCatalogOperation;
  onToggle: (capabilityCode: string, operationCode: string) => void;
  onToggleExpandedOperation: (operationKey: string) => void;
  pageT: (key: string, defaultValue: string, options?: Record<string, unknown>) => string;
  selected: boolean;
}) {
  const operationKey = capabilityGrantKey(capabilityCode, operation.operationCode);
  const checkboxId = checkboxDomId("open-app-operation", capabilityCode, operation.operationCode);

  return (
    <div
      className={`overflow-hidden rounded-2xl border bg-[var(--color-elevated-surface)] transition-[border-color,box-shadow] ${
        selected
          ? "border-[var(--color-outline-variant)] shadow-[0_10px_24px_rgba(15,23,42,0.05)]"
          : "border-[var(--color-card-border)]"
      }`}
    >
      <div
        className={`flex items-center justify-between gap-3 px-4 py-3 transition-colors ${
          selected ? "bg-[var(--color-surface-container-low)]" : ""
        }`}
      >
        <input
          id={checkboxId}
          type="checkbox"
          aria-label={operation.operationName}
          className="size-4 accent-[var(--color-primary)]"
          checked={selected}
          disabled={disabled}
          onChange={() => onToggle(capabilityCode, operation.operationCode)}
        />
        <button
          type="button"
          className="flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl px-2 py-1.5 text-left transition-colors hover:bg-[var(--color-surface-container)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-inset"
          aria-expanded={expanded}
          aria-label={expanded
            ? pageT("capabilityGrants.collapseOperation", "Collapse {{name}}", { name: operation.operationName })
            : pageT("capabilityGrants.expandOperation", "Expand {{name}}", { name: operation.operationName })}
          onClick={() => onToggleExpandedOperation(operationKey)}
        >
          <span className="min-w-0">
            <span className="flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium text-[var(--color-on-surface)]">{operation.operationName}</span>
              <span
                className={`font-mono text-[12px] ${
                  selected ? "text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)]"
                }`}
              >
                {operation.operationCode}
              </span>
            </span>
          </span>
          {expanded ? <ChevronDown size={16} aria-hidden="true" /> : <ChevronRight size={16} aria-hidden="true" />}
        </button>
      </div>

      {expanded ? (
        <div className="border-t border-[var(--color-card-border)] px-4 py-3">
          <ul className="space-y-2">
            {operation.routes.map((route) => (
              <li
                key={route}
                className="rounded-lg bg-[var(--color-surface-container)] px-3 py-2 font-mono text-[11px] leading-5 text-[var(--color-on-surface-variant)]"
              >
                {route}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

export function OpenAppUserGrantSummarySection({
  detail,
}: {
  detail: ManagedOpenAppDetail;
}) {
  const pageT = useOpenAppPageT();

  return (
    <SurfaceCard className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-[var(--color-on-surface)]">
          {pageT("userGrants.summaryTitle", "User Grants")}
        </h3>
      </div>

      {detail.userGrants.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] px-4 py-8 text-center text-sm text-[var(--color-on-surface-variant)]">
          {pageT("userGrants.empty", "No users granted.")}
        </div>
      ) : (
        <div className="flex flex-wrap gap-2">
          {detail.userGrants.map((grant) => (
            <span
              key={grant.userCode}
              className="rounded-full bg-[var(--color-surface-container)] px-3 py-1.5 font-mono text-sm text-[var(--color-on-surface)]"
            >
              {grant.userCode}
            </span>
          ))}
        </div>
      )}
    </SurfaceCard>
  );
}

export function OpenAppCapabilityGrantSummarySection({
  catalog,
  detail,
}: {
  catalog: OpenApiCapabilityCatalogGroup[];
  detail: ManagedOpenAppDetail;
}) {
  const pageT = useOpenAppPageT();
  const grantedGroups = formatGrantedCapabilityGroups(detail.capabilityGrants, catalog);
  const [expandedCapabilityCodes, setExpandedCapabilityCodes] = useState<string[]>([]);
  const [expandedOperationKeys, setExpandedOperationKeys] = useState<string[]>([]);

  useEffect(() => {
    const validCapabilityCodes = new Set(grantedGroups.map((group) => group.capabilityCode));
    const validOperationKeys = new Set(
      grantedGroups.flatMap((group) => group.operations.map((operation) => capabilityGrantKey(group.capabilityCode, operation.operationCode))),
    );
    setExpandedCapabilityCodes((current) => retainMatchingValues(current, validCapabilityCodes));
    setExpandedOperationKeys((current) => retainMatchingValues(current, validOperationKeys));
  }, [grantedGroups]);

  function toggleExpandedCapability(capabilityCode: string) {
    setExpandedCapabilityCodes((current) => (
      current.includes(capabilityCode)
        ? current.filter((code) => code !== capabilityCode)
        : [...current, capabilityCode]
    ));
  }

  function toggleExpandedOperation(operationKey: string) {
    setExpandedOperationKeys((current) => (
      current.includes(operationKey)
        ? current.filter((key) => key !== operationKey)
        : [...current, operationKey]
    ));
  }

  return (
    <SurfaceCard className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-[var(--color-on-surface)]">
          {pageT("capabilityGrants.summaryTitle", "OpenAPI Grants")}
        </h3>
      </div>

      {grantedGroups.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] px-4 py-8 text-center text-sm text-[var(--color-on-surface-variant)]">
          {pageT("capabilityGrants.empty", "No capability grants assigned.")}
        </div>
      ) : (
        <div className="space-y-4">
          {grantedGroups.map((group) => (
            <CapabilityGrantSummaryGroupCard
              key={group.capabilityCode}
              expanded={expandedCapabilityCodes.includes(group.capabilityCode)}
              expandedOperationKeys={expandedOperationKeys}
              group={group}
              pageT={pageT}
              onToggleExpandedCapability={toggleExpandedCapability}
              onToggleExpandedOperation={toggleExpandedOperation}
            />
          ))}
        </div>
      )}
    </SurfaceCard>
  );
}

function CapabilityGrantSummaryGroupCard({
  expanded,
  expandedOperationKeys,
  group,
  onToggleExpandedCapability,
  onToggleExpandedOperation,
  pageT,
}: {
  expanded: boolean;
  expandedOperationKeys: string[];
  group: ReturnType<typeof formatGrantedCapabilityGroups>[number];
  onToggleExpandedCapability: (capabilityCode: string) => void;
  onToggleExpandedOperation: (operationKey: string) => void;
  pageT: (key: string, defaultValue: string, options?: Record<string, unknown>) => string;
}) {
  const capabilityDisplayName = formatCapabilityDisplayName(group.capabilityName);

  return (
    <div className="overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-lowest)]">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left transition-colors hover:bg-[var(--color-surface-container)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-inset"
        aria-expanded={expanded}
        aria-label={expanded
          ? pageT("capabilityGrants.collapseCapability", "Collapse {{name}}", { name: capabilityDisplayName })
          : pageT("capabilityGrants.expandCapability", "Expand {{name}}", { name: capabilityDisplayName })}
        onClick={() => onToggleExpandedCapability(group.capabilityCode)}
      >
        <span className="min-w-0 flex-1">
          <span className="block text-base font-semibold text-[var(--color-on-surface)]">
            {capabilityDisplayName}
          </span>
          {!expanded ? (
            <span className="mt-2 flex flex-wrap gap-2">
              {group.operations.map((operation) => (
                <span
                  key={`${group.capabilityCode}:${operation.operationCode}:summary`}
                  className="rounded-full bg-[var(--color-surface-container)] px-2.5 py-1 text-xs font-medium text-[var(--color-on-surface-variant)]"
                >
                  {operation.operationName}
                </span>
              ))}
            </span>
          ) : null}
        </span>
        {expanded ? <ChevronDown size={16} aria-hidden="true" /> : <ChevronRight size={16} aria-hidden="true" />}
      </button>

      {expanded ? (
        <div className="border-t border-[var(--color-card-border)] px-3 py-3">
          <div className="space-y-2">
            {group.operations.map((operation) => (
              <CapabilityGrantSummaryOperationCard
                key={`${group.capabilityCode}:${operation.operationCode}`}
                capabilityCode={group.capabilityCode}
                expanded={expandedOperationKeys.includes(capabilityGrantKey(group.capabilityCode, operation.operationCode))}
                operation={operation}
                pageT={pageT}
                onToggleExpandedOperation={onToggleExpandedOperation}
              />
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function CapabilityGrantSummaryOperationCard({
  capabilityCode,
  expanded,
  operation,
  onToggleExpandedOperation,
  pageT,
}: {
  capabilityCode: string;
  expanded: boolean;
  operation: {
    description: string;
    operationCode: string;
    operationName: string;
    routes: string[];
  };
  onToggleExpandedOperation: (operationKey: string) => void;
  pageT: (key: string, defaultValue: string, options?: Record<string, unknown>) => string;
}) {
  const operationKey = capabilityGrantKey(capabilityCode, operation.operationCode);

  return (
    <div className="overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left transition-colors hover:bg-[var(--color-surface-container)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-inset"
        aria-expanded={expanded}
        aria-label={expanded
          ? pageT("capabilityGrants.collapseOperation", "Collapse {{name}}", { name: operation.operationName })
          : pageT("capabilityGrants.expandOperation", "Expand {{name}}", { name: operation.operationName })}
        onClick={() => onToggleExpandedOperation(operationKey)}
      >
        <span className="min-w-0">
          <span className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium text-[var(--color-on-surface)]">{operation.operationName}</span>
            <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">{operation.operationCode}</span>
          </span>
        </span>
        {expanded ? <ChevronDown size={16} aria-hidden="true" /> : <ChevronRight size={16} aria-hidden="true" />}
      </button>

      {expanded ? (
        <div className="border-t border-[var(--color-card-border)] px-4 py-3">
          <ul className="space-y-2">
            {operation.routes.map((route) => (
              <li
                key={route}
                className="rounded-lg bg-[var(--color-surface-container)] px-3 py-2 font-mono text-[11px] leading-5 text-[var(--color-on-surface-variant)]"
              >
                {route}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
