import {
  type FormEvent,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  useNavigate,
  useParams,
} from "react-router";
import { useTranslation } from "react-i18next";
import {
  Plus,
  Save,
  Trash2,
  X,
} from "lucide-react";

import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useToast,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  getRagMockCorpora,
  saveRagMockCorpus,
} from "@/domain/rag/mock/data";
import {
  buildRagCorpusMutationPayload,
  createRagCorpus,
  fetchRagCorpusCatalog,
  normalizeRagValidationIssues,
  updateRagCorpus,
  validationIssueField,
  validationIssueMessage,
  verifyRagCorpus,
  type RagManagementValidationIssue,
} from "@/domain/rag/api/ragManagementApi";
import { useRagCorpusDetail } from "@/domain/rag/hooks/useRagCorpusDetail";
import {
  buildRagCorpusPayloadPreview,
  buildRagCorpusStorageBinding,
  buildCorpusRecordFromForm,
  buildRagCorpusFormState,
  editableStorageSegmentOrder,
  getGraphCapabilityClosureError,
  hasRagCorpusFormErrors,
  listAvailableFilterFieldOptions,
  type RagEditableStorageSegmentKey,
  type RagFieldMappingFormRow,
  type RagFilterFieldFormRow,
  type RagCorpusFormErrors,
  type RagCorpusFormState,
  type RagStorageTargetFormErrors,
  type RagStorageTargetFormState,
  ragStorageTargetFormDefaults,
  ragStorageTargetOptionLabel,
  validateRagCorpusForm,
} from "@/domain/rag/model/ragCorpusForm";
import { formatRagStorageBinding } from "@/domain/rag/model/RagModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const segmentBindingMeta: Array<{
  key: RagEditableStorageSegmentKey;
  label: string;
}> = [
  {
    key: "source",
    label: "Source",
  },
  {
    key: "textUnit",
    label: "Text unit",
  },
  {
    key: "fulltext",
    label: "Fulltext index",
  },
  {
    key: "vector",
    label: "Vector",
  },
  {
    key: "tree",
    label: "Summary tree",
  },
];

const corpusFieldHints = {
  code: "Stable corpus code used in API parameters and profile references. It cannot be changed after creation.",
  description: "Purpose description used to identify this corpus in lists and profile pages.",
  filterName: "Business field allowed for retrieval profiles and workbench filters. It must come from field mappings.",
  filterOperators: "Supported filter operators for this field. Separate multiple operators with commas, such as eq, in.",
  filterType: "Filter value type used when saving configuration and validating workbench test input.",
  mappingColumn: "Column or property name used when writing this field to the underlying storage.",
  mappingLogicalName: "Business-facing field name used by filter schema, workbench attributes, and return fields.",
  name: "Display name shown to users.",
  segmentBinding: "Storage target used by this data segment. Empty means the segment is not bound.",
  storageBackend: "Choose local table storage or graph storage for this target. Each backend requires different fields.",
  storageCodeStrategy: "Graph node code generation strategy that keeps node codes stable for the same source version and scope.",
  storageNamespace: "Graph storage namespace used to isolate corpora or business domains.",
  storageNodeLabel: "Graph node label used to identify text-unit or document nodes.",
  storageTable: "Table name used by local table storage. Auto-generated values can be overridden.",
} satisfies Record<string, string>;

function hasGraphStorageTarget(storageTargets: RagStorageTargetFormState[]) {
  return storageTargets.some((target) => target.backend === "GRAPH");
}

function normalizeGraphCapability(form: RagCorpusFormState) {
  if (hasGraphStorageTarget(form.storageTargets)) return form;
  return {
    ...form,
    graphEnabled: false,
  };
}

function useCorpusFormCopy() {
  const { t } = useTranslation("pages");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.corpusForm.${key}`, { defaultValue, ...options })
  );
  const hintT = (key: keyof typeof corpusFieldHints) => pageT(`hints.${key}`, corpusFieldHints[key]);
  return { pageT, t, hintT };
}

function StorageTargetCard({
  canRemove,
  error,
  index,
  removeTarget,
  target,
  updateTarget,
}: {
  canRemove: boolean;
  error?: RagStorageTargetFormErrors;
  index: number;
  removeTarget: (formId: string) => void;
  target: RagStorageTargetFormState;
  updateTarget: (formId: string, patch: Partial<RagStorageTargetFormState>) => void;
}) {
  const { pageT, hintT } = useCorpusFormCopy();
  const showInlineRemove = canRemove;

  return (
    <div className="space-y-4 rounded-xl border border-[var(--color-outline-variant)] p-4">
      {target.backend === "SQLITE" ? (
        <div className={`grid gap-4 ${showInlineRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] lg:items-end" : "lg:grid-cols-2"}`}>
          <Field label={pageT("fields.backend", "Backend")} error={error?.backend} hint={hintT("storageBackend")}>
            <SelectInput
              aria-label={pageT("storageTargets.backendAria", "Storage target {{index}} backend", { index: index + 1 })}
              className="w-full"
              value={target.backend}
              onChange={(event) => updateTarget(target.formId, { backend: event.target.value as RagStorageTargetFormState["backend"] })}
            >
              <option value="SQLITE">{pageT("storageTargets.sqlite", "Local table storage")}</option>
              <option value="GRAPH">{pageT("storageTargets.graph", "Graph storage")}</option>
            </SelectInput>
          </Field>
          <Field label={pageT("fields.table", "Table")} error={error?.table} hint={hintT("storageTable")}>
            <TextInput
              aria-label={pageT("storageTargets.tableAria", "Storage target {{index}} table", { index: index + 1 })}
              className="w-full font-mono"
              value={target.table}
              onChange={(event) => updateTarget(target.formId, {
                table: event.target.value,
                tableAutoGenerated: false,
              })}
            />
          </Field>
          {showInlineRemove ? (
            <div className="flex items-center justify-end lg:pb-1">
              <Button
                aria-label={pageT("storageTargets.removeAria", "Remove storage target {{index}}", { index: index + 1 })}
                size="icon"
                type="button"
                variant="ghost"
                onClick={() => removeTarget(target.formId)}
              >
                <Trash2 size={16} aria-hidden="true" />
              </Button>
            </div>
          ) : null}
        </div>
      ) : (
        <>
          <div className={`grid gap-4 ${showInlineRemove ? "lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] lg:items-end" : "lg:grid-cols-2"}`}>
            <Field label={pageT("fields.backend", "Backend")} error={error?.backend} hint={hintT("storageBackend")}>
              <SelectInput
                aria-label={pageT("storageTargets.backendAria", "Storage target {{index}} backend", { index: index + 1 })}
                className="w-full"
                value={target.backend}
                onChange={(event) => updateTarget(target.formId, { backend: event.target.value as RagStorageTargetFormState["backend"] })}
              >
                <option value="SQLITE">{pageT("storageTargets.sqlite", "Local table storage")}</option>
                <option value="GRAPH">{pageT("storageTargets.graph", "Graph storage")}</option>
              </SelectInput>
            </Field>
            <Field label={pageT("fields.graphNamespace", "Graph namespace")} error={error?.namespace} hint={hintT("storageNamespace")}>
              <TextInput
                aria-label={pageT("storageTargets.namespaceAria", "Storage target {{index}} graph namespace", { index: index + 1 })}
                className="w-full font-mono"
                value={target.namespace}
                onChange={(event) => updateTarget(target.formId, { namespace: event.target.value })}
              />
            </Field>
            {showInlineRemove ? (
              <div className="flex items-center justify-end lg:pb-1">
                <Button
                  aria-label={pageT("storageTargets.removeAria", "Remove storage target {{index}}", { index: index + 1 })}
                  size="icon"
                  type="button"
                  variant="ghost"
                  onClick={() => removeTarget(target.formId)}
                >
                  <Trash2 size={16} aria-hidden="true" />
                </Button>
              </div>
            ) : null}
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            <Field label={pageT("fields.nodeLabel", "Node label")} error={error?.nodeLabel} hint={hintT("storageNodeLabel")}>
              <TextInput
                aria-label={pageT("storageTargets.nodeLabelAria", "Storage target {{index}} node label", { index: index + 1 })}
                className="w-full font-mono"
                value={target.nodeLabel}
                onChange={(event) => updateTarget(target.formId, { nodeLabel: event.target.value })}
              />
            </Field>
            <Field label={pageT("fields.codeStrategy", "Code strategy")} hint={hintT("storageCodeStrategy")}>
              <TextInput
                aria-label={pageT("storageTargets.codeStrategyAria", "Storage target {{index}} code strategy", { index: index + 1 })}
                className="w-full font-mono"
                value={target.codeStrategy}
                onChange={(event) => updateTarget(target.formId, { codeStrategy: event.target.value })}
              />
            </Field>
          </div>
        </>
      )}
    </div>
  );
}

function FieldMappingRowCard({
  index,
  mappingKey,
  row,
  onRemove,
  onUpdate,
}: {
  index: number;
  mappingKey: "sourceMappings" | "textUnitMappings";
  row: RagFieldMappingFormRow;
  onRemove: () => void;
  onUpdate: (patch: Partial<RagFieldMappingFormRow>) => void;
}) {
  const { pageT, hintT } = useCorpusFormCopy();
  const isSource = mappingKey === "sourceMappings";
  const rowLabelPrefix = row.isContract
    ? (isSource ? pageT("mapping.sourceContractField", "Source contract field") : pageT("mapping.textUnitContractField", "Text unit contract field"))
    : (isSource ? pageT("mapping.sourceMapping", "Source mapping") : pageT("mapping.textUnitMapping", "Text unit mapping"));
  const ariaPrefix = rowLabelPrefix;
  const showInlineRemove = !row.isContract;

  return (
    <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      {row.isContract ? (
        <div className="flex justify-end">
          <span className="rounded-full border border-[var(--color-outline-variant)] px-2 py-0.5 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
            {pageT("mapping.contract", "Contract")}
          </span>
        </div>
      ) : null}
      <div className={`grid gap-3 ${showInlineRemove ? "sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end" : "sm:grid-cols-2"}`}>
        <Field label={pageT("fields.logicalName", "Logical name")} hint={hintT("mappingLogicalName")}>
          <TextInput
            aria-label={pageT("mapping.logicalNameAria", "{{prefix}} {{index}} logical name", { index: index + 1, prefix: ariaPrefix })}
            className="w-full font-mono"
            disabled={row.isContract}
            value={row.logicalName}
            onChange={(event) => onUpdate({ logicalName: event.target.value })}
          />
        </Field>
        <Field label={pageT("fields.column", "Column")} hint={hintT("mappingColumn")}>
          <TextInput
            aria-label={pageT("mapping.columnAria", "{{prefix}} {{index}} column", { index: index + 1, prefix: ariaPrefix })}
            className="w-full font-mono"
            value={row.column}
            onChange={(event) => onUpdate({ column: event.target.value })}
          />
        </Field>
        {showInlineRemove ? (
          <div className="flex items-center justify-end sm:pb-1">
            <Button
              aria-label={pageT("mapping.removeAria", "Remove {{prefix}} {{index}}", { index: index + 1, prefix: ariaPrefix })}
              size="icon"
              type="button"
              variant="ghost"
              onClick={onRemove}
            >
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          </div>
        ) : null}
      </div>
    </div>
  );
}

function FieldMappingSection({
  addButtonLabel,
  contractRows,
  customRows,
  error,
  mappingKey,
  onAdd,
  onRemove,
  onToggleContractFields,
  onUpdate,
  showContractRows,
  title,
  toggleContractLabel,
}: {
  addButtonLabel: string;
  contractRows: Array<{ index: number; row: RagFieldMappingFormRow }>;
  customRows: Array<{ index: number; row: RagFieldMappingFormRow }>;
  error?: string;
  mappingKey: "sourceMappings" | "textUnitMappings";
  onAdd: () => void;
  onRemove: (index: number) => void;
  onToggleContractFields: () => void;
  onUpdate: (index: number, patch: Partial<RagFieldMappingFormRow>) => void;
  showContractRows: boolean;
  title: string;
  toggleContractLabel: string;
}) {
  const { pageT } = useCorpusFormCopy();

  return (
    <div className="space-y-4 rounded-xl border border-[var(--color-outline-variant)] p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <h3 className="text-sm font-semibold">{title}</h3>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="ghost" onClick={onToggleContractFields}>
            {toggleContractLabel}
          </Button>
          <Button type="button" variant="secondary" onClick={onAdd}>{addButtonLabel}</Button>
        </div>
      </div>

      {showContractRows ? (
        <div className="space-y-3">
          <h4 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
            {pageT("mapping.contractFields", "Contract fields")}
          </h4>
          {contractRows.map(({ index, row }, contractIndex) => (
          <FieldMappingRowCard
            key={row.id}
            index={contractIndex}
            mappingKey={mappingKey}
            row={row}
            onRemove={() => onRemove(index)}
            onUpdate={(patch) => onUpdate(index, patch)}
          />
          ))}
        </div>
      ) : null}

      <div className="space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
          {pageT("mapping.customMappings", "Custom mappings")}
        </h4>
        {customRows.length ? customRows.map(({ index, row }, customIndex) => (
          <FieldMappingRowCard
            key={row.id}
            index={customIndex}
            mappingKey={mappingKey}
            row={row}
            onRemove={() => onRemove(index)}
            onUpdate={(patch) => onUpdate(index, patch)}
          />
        )) : (
          <div className="rounded-lg border border-dashed border-[var(--color-outline-variant)] px-3 py-4 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("mapping.noCustomMappings", "No custom mappings.")}
          </div>
        )}
      </div>

      {error ? <p className="text-[12px] text-[var(--color-error)]">{error}</p> : null}
    </div>
  );
}

function mapCorpusApiErrors(
  issues: RagManagementValidationIssue[],
  form: RagCorpusFormState,
): RagCorpusFormErrors {
  const nextErrors: RagCorpusFormErrors = {};

  issues.forEach((issue) => {
    const field = validationIssueField(issue);
    const message = validationIssueMessage(issue);

    if (field === "code" || field === "corpusId") {
      nextErrors.code = nextErrors.code ?? message;
      return;
    }

    if (field === "name" || field === "displayName") {
      nextErrors.name = nextErrors.name ?? message;
      return;
    }

    if (field === "storageBinding") {
      nextErrors.storageBinding = nextErrors.storageBinding ?? message;
      return;
    }

    if (field.startsWith("segmentBindings.")) {
      const segmentKey = field.replace("segmentBindings.", "") as RagEditableStorageSegmentKey;
      nextErrors.segmentBindings = {
        ...nextErrors.segmentBindings,
        [segmentKey]: nextErrors.segmentBindings?.[segmentKey] ?? message,
      };
      return;
    }

    const storageTargetMatch = field.match(/^storageTargets\[(\d+)\]\.(backend|namespace|nodeLabel|table)$/);
    if (storageTargetMatch) {
      const index = Number(storageTargetMatch[1]);
      const targetField = storageTargetMatch[2] as keyof RagStorageTargetFormErrors;
      const target = form.storageTargets[index];
      if (!target) return;
      nextErrors.storageTargets = {
        ...(nextErrors.storageTargets ?? {}),
        [target.formId]: {
          ...(nextErrors.storageTargets?.[target.formId] ?? {}),
          [targetField]: message,
        },
      };
      return;
    }

    if (field.startsWith("sourceMappings")) {
      nextErrors.sourceMappings = nextErrors.sourceMappings ?? message;
      return;
    }

    if (field.startsWith("textUnitMappings")) {
      nextErrors.textUnitMappings = nextErrors.textUnitMappings ?? message;
      return;
    }

    if (field.startsWith("filter")) {
      nextErrors.filterFields = nextErrors.filterFields ?? message;
    }
  });

  return nextErrors;
}

export function RagCorpusFormPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { pageT, hintT } = useCorpusFormCopy();
  const dataSource = getServiceDataSource("rag");
  const isApiMode = dataSource === "api";
  const { corpusCode } = useParams<{ corpusCode: string }>();
  const activeCorpusCode = corpusCode ?? "";
  const mode = activeCorpusCode ? "edit" : "create";
  const { detail: apiDetail, loading: detailLoading, record: detailRecord } = useRagCorpusDetail(activeCorpusCode);
  const existingCorpus = detailRecord ?? undefined;
  const [form, setForm] = useState<RagCorpusFormState>(() => normalizeGraphCapability(buildRagCorpusFormState()));
  const [errors, setErrors] = useState<RagCorpusFormErrors>({});
  const [catalogLoading, setCatalogLoading] = useState(isApiMode);
  const [metadataGroups, setMetadataGroups] = useState<Array<Record<string, unknown>>>([]);
  const [submitting, setSubmitting] = useState(false);
  const [showSourceContractMappings, setShowSourceContractMappings] = useState(false);
  const [showTextUnitContractMappings, setShowTextUnitContractMappings] = useState(false);
  const existingCodes = useMemo(
    () => (isApiMode ? [] : getRagMockCorpora().map((corpus) => corpus.code)),
    [isApiMode],
  );
  const title = mode === "create" ? pageT("title.create", "New corpus") : pageT("title.edit", "Edit corpus");
  const payloadPreview = useMemo(
    () => (isApiMode ? buildRagCorpusMutationPayload(form, metadataGroups) : buildRagCorpusPayloadPreview(form)),
    [form, isApiMode, metadataGroups],
  );
  const filterFieldOptions = useMemo(() => listAvailableFilterFieldOptions(form), [form]);
  const graphCapabilityAvailable = useMemo(() => hasGraphStorageTarget(form.storageTargets), [form.storageTargets]);
  const visibleSourceMappings = useMemo(
    () => form.sourceMappings
      .map((row, index) => ({ index, row }))
      .filter(({ row }) => showSourceContractMappings || !row.isContract),
    [form.sourceMappings, showSourceContractMappings],
  );
  const visibleTextUnitMappings = useMemo(
    () => form.textUnitMappings
      .map((row, index) => ({ index, row }))
      .filter(({ row }) => showTextUnitContractMappings || !row.isContract),
    [form.textUnitMappings, showTextUnitContractMappings],
  );
  const sourceContractMappings = useMemo(
    () => visibleSourceMappings.filter(({ row }) => row.isContract),
    [visibleSourceMappings],
  );
  const sourceCustomMappings = useMemo(
    () => visibleSourceMappings.filter(({ row }) => !row.isContract),
    [visibleSourceMappings],
  );
  const textUnitContractMappings = useMemo(
    () => visibleTextUnitMappings.filter(({ row }) => row.isContract),
    [visibleTextUnitMappings],
  );
  const textUnitCustomMappings = useMemo(
    () => visibleTextUnitMappings.filter(({ row }) => !row.isContract),
    [visibleTextUnitMappings],
  );
  const storageBindingSectionError = useMemo(
    () => errors.storageBinding ?? getGraphCapabilityClosureError(form),
    [errors.storageBinding, form],
  );
  const loading = isApiMode && (catalogLoading || (mode === "edit" && detailLoading && !existingCorpus));

  useEffect(() => {
    if (!isApiMode) {
      setCatalogLoading(false);
      return;
    }

    const controller = new AbortController();
    setCatalogLoading(true);

    fetchRagCorpusCatalog(controller.signal)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error ? error.message : pageT("messages.catalogLoadFailed", "Corpus catalog could not be loaded."));
      })
      .finally(() => {
        if (!controller.signal.aborted) setCatalogLoading(false);
      });

    return () => controller.abort();
  }, [isApiMode, toast]);

  useEffect(() => {
    if (mode === "create") {
      setForm(normalizeGraphCapability(buildRagCorpusFormState()));
      if (isApiMode) setMetadataGroups([]);
      return;
    }

    if (!existingCorpus) return;
    setForm(normalizeGraphCapability(buildRagCorpusFormState(existingCorpus)));
    if (isApiMode) {
      setMetadataGroups(Array.isArray(apiDetail?.metadataGroups) ? apiDetail.metadataGroups : []);
    }
  }, [apiDetail?.corpusId, existingCorpus, isApiMode, mode]);

  function deriveStorageBindingValue(nextForm: RagCorpusFormState) {
    return formatRagStorageBinding(buildRagCorpusStorageBinding(nextForm));
  }

  function clearTargetError(currentErrors: RagCorpusFormErrors, formId: string) {
    if (!currentErrors.storageTargets?.[formId]) return currentErrors;
    const nextStorageTargets = { ...currentErrors.storageTargets };
    delete nextStorageTargets[formId];
    return {
      ...currentErrors,
      storageTargets: nextStorageTargets,
    };
  }

  function updateForm<K extends keyof RagCorpusFormState>(key: K, value: RagCorpusFormState[K]) {
    setForm((currentForm) => {
      let nextForm = { ...currentForm, [key]: value };
      if (key === "code" || key === "name") {
        nextForm.storageTargets = nextForm.storageTargets.map((target, index) => {
          if (target.backend === "GRAPH" && target.id === ragStorageTargetFormDefaults.defaultSegmentBindings.tree) {
            return {
              ...target,
              namespace: ragStorageTargetFormDefaults.deriveGraphTargetNamespace(target, nextForm.code, nextForm.name),
            };
          }
          if (target.backend !== "SQLITE" || !target.tableAutoGenerated) return target;
          return {
            ...target,
            table: ragStorageTargetFormDefaults.deriveSqliteTargetTable(target, nextForm.code, nextForm.name, index),
          };
        });
      }
      nextForm = normalizeGraphCapability(nextForm);
      nextForm.storageBinding = deriveStorageBindingValue(nextForm);
      return nextForm;
    });
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      if (key === "graphEnabled") {
        delete nextErrors.storageBinding;
      }
      if (!currentErrors[key as keyof RagCorpusFormErrors]) {
        return Object.keys(nextErrors).length === Object.keys(currentErrors).length ? currentErrors : nextErrors;
      }
      delete nextErrors[key as keyof RagCorpusFormErrors];
      return nextErrors;
    });
  }

  function updateTarget(formId: string, patch: Partial<RagStorageTargetFormState>) {
    setForm((currentForm) => {
      const currentTargetIndex = currentForm.storageTargets.findIndex((target) => target.formId === formId);
      const currentTarget = currentTargetIndex >= 0 ? currentForm.storageTargets[currentTargetIndex] : undefined;
      if (!currentTarget) return currentForm;

      const nextTarget = {
        ...currentTarget,
        ...patch,
      };

      if (patch.backend === "GRAPH") {
        nextTarget.namespace = nextTarget.namespace || ragStorageTargetFormDefaults.deriveGraphTargetNamespace(nextTarget, currentForm.code, currentForm.name);
        nextTarget.nodeLabel = nextTarget.nodeLabel || ragStorageTargetFormDefaults.deriveGraphTargetNodeLabel(nextTarget);
        nextTarget.codeStrategy = nextTarget.codeStrategy || "CORPUS_SOURCE_VERSION_SCOPE_HASH";
      }

      if (patch.backend === "SQLITE") {
        nextTarget.table = nextTarget.tableAutoGenerated || !nextTarget.table.trim()
          ? ragStorageTargetFormDefaults.deriveSqliteTargetTable(nextTarget, currentForm.code, currentForm.name, currentTargetIndex)
          : nextTarget.table;
      }

      let nextForm = {
        ...currentForm,
        storageTargets: currentForm.storageTargets.map((target) => (
          target.formId === formId ? nextTarget : target
        )),
      };
      nextForm = normalizeGraphCapability(nextForm);
      nextForm.storageBinding = deriveStorageBindingValue(nextForm);
      return nextForm;
    });

    setErrors((currentErrors) => {
      let nextErrors = clearTargetError(currentErrors, formId);
      if (nextErrors === currentErrors) {
        nextErrors = { ...currentErrors };
      }
      delete nextErrors.storageBinding;
      return nextErrors;
    });
  }

  function addStorageTarget() {
    setForm((currentForm) => {
      const nextFormId = ragStorageTargetFormDefaults.nextStorageTargetFormId(currentForm.storageTargets);
      const nextTarget: RagStorageTargetFormState = {
        backend: "SQLITE",
        codeStrategy: "",
        formId: nextFormId,
        id: `target-${nextFormId.replace("target-form-", "") || currentForm.storageTargets.length + 1}`,
        namespace: ragStorageTargetFormDefaults.deriveGraphTargetNamespace({ id: "" }, currentForm.code, currentForm.name),
        nodeLabel: ragStorageTargetFormDefaults.deriveGraphTargetNodeLabel({ id: "" }),
        table: "",
        tableAutoGenerated: true,
      };
      nextTarget.table = ragStorageTargetFormDefaults.deriveSqliteTargetTable(
        nextTarget,
        currentForm.code,
        currentForm.name,
        currentForm.storageTargets.length,
      );
      let nextForm = {
        ...currentForm,
        storageTargets: [...currentForm.storageTargets, nextTarget],
      };
      nextForm = normalizeGraphCapability(nextForm);
      nextForm.storageBinding = deriveStorageBindingValue(nextForm);
      return nextForm;
    });
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors.storageBinding;
      return nextErrors;
    });
  }

  function removeStorageTarget(formId: string) {
    setForm((currentForm) => {
      if (currentForm.storageTargets.length <= 1) return currentForm;
      const removedTarget = currentForm.storageTargets.find((target) => target.formId === formId);
      if (!removedTarget) return currentForm;

      const removedTargetId = removedTarget.id.trim();
      let nextForm = {
        ...currentForm,
        segmentBindings: Object.fromEntries(
          Object.entries(currentForm.segmentBindings).map(([segmentKey, targetId]) => (
            [segmentKey, targetId === removedTargetId ? "" : targetId]
          )),
        ) as RagCorpusFormState["segmentBindings"],
        storageTargets: currentForm.storageTargets.filter((target) => target.formId !== formId),
      };
      nextForm = normalizeGraphCapability(nextForm);
      nextForm.storageBinding = deriveStorageBindingValue(nextForm);
      return nextForm;
    });

    setErrors((currentErrors) => {
      const nextErrors = clearTargetError(currentErrors, formId);
      delete nextErrors.storageBinding;
      return nextErrors;
    });
  }

  function updateSegmentBinding(segmentKey: RagEditableStorageSegmentKey, targetId: string) {
    setForm((currentForm) => {
      const nextForm = {
        ...currentForm,
        segmentBindings: {
          ...currentForm.segmentBindings,
          [segmentKey]: targetId,
        },
      };
      nextForm.storageBinding = deriveStorageBindingValue(nextForm);
      return nextForm;
    });
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors.storageBinding;
      if (!currentErrors.segmentBindings?.[segmentKey]) return nextErrors;
      nextErrors.segmentBindings = {
        ...currentErrors.segmentBindings,
        [segmentKey]: undefined,
      };
      return nextErrors;
    });
  }

  function updateFilterField(index: number, patch: Partial<RagFilterFieldFormRow>) {
    setForm((currentForm) => ({
      ...currentForm,
      filterFields: currentForm.filterFields.map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row)),
      filtersInput: currentForm.filterFields
        .map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row))
        .map((row) => row.name.trim())
        .filter(Boolean)
        .join(", "),
    }));
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors.filterFields;
      delete nextErrors.filtersInput;
      return nextErrors;
    });
  }

  function addFilterField() {
    setForm((currentForm) => ({
      ...currentForm,
      filterFields: [
        ...currentForm.filterFields,
        {
          id: ragStorageTargetFormDefaults.nextFormRowId(currentForm.filterFields, "filter"),
          name: "",
          operatorsInput: "eq, in",
          type: "string",
        },
      ],
    }));
  }

  function removeFilterField(index: number) {
    setForm((currentForm) => {
      const nextFilterFields = currentForm.filterFields.filter((_, rowIndex) => rowIndex !== index);
      return {
        ...currentForm,
        filterFields: nextFilterFields,
        filtersInput: nextFilterFields
          .map((row) => row.name.trim())
          .filter(Boolean)
          .join(", "),
      };
    });
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors.filterFields;
      delete nextErrors.filtersInput;
      return nextErrors;
    });
  }

  function updateMapping(
    key: "sourceMappings" | "textUnitMappings",
    index: number,
    patch: Partial<RagFieldMappingFormRow>,
  ) {
    setForm((currentForm) => ({
      ...currentForm,
      [key]: currentForm[key].map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row)),
    }));
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors[key];
      return nextErrors;
    });
  }

  function addMapping(key: "sourceMappings" | "textUnitMappings") {
    setForm((currentForm) => ({
      ...currentForm,
      [key]: [
        ...currentForm[key],
        {
          column: "",
          id: ragStorageTargetFormDefaults.nextFormRowId(currentForm[key], key === "sourceMappings" ? "source-map" : "text-unit-map"),
          isContract: false,
          logicalName: "",
        },
      ],
    }));
  }

  function removeMapping(key: "sourceMappings" | "textUnitMappings", index: number) {
    setForm((currentForm) => ({
      ...currentForm,
      [key]: currentForm[key].filter((row, rowIndex) => rowIndex !== index || row.isContract),
    }));
    setErrors((currentErrors) => {
      const nextErrors = { ...currentErrors };
      delete nextErrors[key];
      return nextErrors;
    });
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const nextErrors = validateRagCorpusForm(form, {
      currentCode: existingCorpus?.code,
      existingCodes,
      mode,
    });

    if (hasRagCorpusFormErrors(nextErrors)) {
      setErrors(nextErrors);
      toast.error(pageT("messages.fixErrors", "Resolve corpus form errors before saving."), { durationMs: 5000 });
      return;
    }

    if (!isApiMode) {
      const savedCorpus = saveRagMockCorpus(buildCorpusRecordFromForm(form, existingCorpus));
      toast.success(pageT(mode === "create" ? "messages.created" : "messages.saved", mode === "create" ? "{{name}} created" : "{{name}} saved", { name: savedCorpus.name }), { durationMs: 6000 });
      navigate("/rag");
      return;
    }

    setSubmitting(true);
    try {
      const payload = buildRagCorpusMutationPayload(form, metadataGroups);
      const verification = await verifyRagCorpus(payload);
      const issues = normalizeRagValidationIssues(verification.errors);

      if (verification.valid === false || issues.length > 0) {
        const apiErrors = mapCorpusApiErrors(issues, form);
        setErrors(apiErrors);
        toast.error(validationIssueMessage(issues[0] ?? { message: pageT("messages.fixErrors", "Resolve corpus form errors before saving.") }), { durationMs: 5000 });
        return;
      }

      const savedCorpus = mode === "create"
        ? await createRagCorpus(payload)
        : await updateRagCorpus(activeCorpusCode, payload);
      const savedName = typeof savedCorpus.displayName === "string" && savedCorpus.displayName.trim()
        ? savedCorpus.displayName.trim()
        : (form.name.trim() || form.code.trim());
      setMetadataGroups(Array.isArray(savedCorpus.metadataGroups) ? savedCorpus.metadataGroups : metadataGroups);
      toast.success(pageT(mode === "create" ? "messages.created" : "messages.saved", mode === "create" ? "{{name}} created" : "{{name}} saved", { name: savedName }), { durationMs: 6000 });
      navigate("/rag");
    } catch (error) {
      const errorData = typeof error === "object" && error !== null && "data" in error
        ? (error as { data?: unknown }).data
        : undefined;
      const issues = typeof errorData === "object" && errorData !== null && "errors" in errorData
        ? normalizeRagValidationIssues((errorData as { errors?: unknown }).errors)
        : [];
      if (issues.length > 0) {
        setErrors(mapCorpusApiErrors(issues, form));
      }
      toast.error(error instanceof Error ? error.message : pageT("messages.saveFailed", "Corpus could not be saved."));
    } finally {
      setSubmitting(false);
    }
  }

  if (mode === "edit" && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "RAG", href: "/rag" }, { label: pageT("loading.breadcrumb", "Loading corpus") }]}
          title={pageT("loading.title", "Loading corpus")}
          actions={<Button variant="secondary" onClick={() => navigate("/rag")}>{pageT("actions.backToList", "Back to list")}</Button>}
        />
        <SurfaceCard>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("loading.description", "Loading corpus management record...")}</p>
        </SurfaceCard>
      </>
    );
  }

  if (mode === "edit" && !existingCorpus) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "RAG", href: "/rag" }, { label: pageT("notFound.breadcrumb", "Corpus not found") }]}
          title={pageT("notFound.title", "Corpus not found")}
          actions={<Button variant="secondary" onClick={() => navigate("/rag")}>{pageT("actions.backToList", "Back to list")}</Button>}
        />
        <SurfaceCard>
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {isApiMode
              ? pageT("notFound.api", "The requested corpus could not be loaded from the RAG management API.")
              : pageT("notFound.mock", "The selected corpus does not exist in the browser-local mock registry.")}
          </p>
        </SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "RAG", href: "/rag" }, { label: mode === "create" ? pageT("title.create", "New corpus") : form.code }]}
        title={title}
        actions={(
          <>
            <Button disabled={submitting || loading} type="button" variant="secondary" onClick={() => navigate("/rag")}>
              <X size={15} aria-hidden="true" />
              {pageT("actions.cancel", "Cancel")}
            </Button>
            <Button aria-busy={submitting} disabled={submitting || loading} form="rag-corpus-form" type="submit">
              <Save size={15} aria-hidden="true" />
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <form id="rag-corpus-form" className="space-y-5" onSubmit={handleSubmit}>
        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold">{pageT("sections.basicInfo", "Basic information")}</h2>
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            <Field label={pageT("fields.corpusCode", "Corpus code")} error={errors.code} hint={hintT("code")}>
              <TextInput
                aria-label={pageT("fields.corpusCode", "Corpus code")}
                className="w-full font-mono"
                disabled={mode === "edit"}
                value={form.code}
                onChange={(event) => updateForm("code", event.target.value)}
              />
            </Field>
            <Field label={pageT("fields.corpusName", "Corpus name")} error={errors.name} hint={hintT("name")}>
              <TextInput
                aria-label={pageT("fields.corpusName", "Corpus name")}
                className="w-full"
                value={form.name}
                onChange={(event) => updateForm("name", event.target.value)}
              />
            </Field>
            <div className="lg:col-span-2">
              <Field label={pageT("fields.description", "Description")} hint={hintT("description")}>
                <TextArea
                  aria-label={pageT("fields.description", "Description")}
                  className="min-h-24 w-full"
                  value={form.description}
                  onChange={(event) => updateForm("description", event.target.value)}
                />
              </Field>
            </div>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold">{pageT("sections.storageBinding", "Storage binding")}</h2>
            {storageBindingSectionError ? (
              <p
                aria-live="polite"
                className="mt-3 rounded-lg border border-[var(--color-error)]/20 bg-[var(--color-error)]/5 px-3 py-2 text-sm text-[var(--color-error)]"
                role="alert"
              >
                {storageBindingSectionError}
              </p>
            ) : null}
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold">{pageT("sections.storageTargets", "Storage targets")}</h3>
              <Button
                type="button"
                variant="secondary"
                onClick={addStorageTarget}
              >
                <Plus size={15} aria-hidden="true" />
                {pageT("actions.addStorageTarget", "Add storage target")}
              </Button>
            </div>

            <div className="space-y-4">
              {form.storageTargets.map((target, index) => (
                <StorageTargetCard
                  key={target.formId}
                  canRemove={form.storageTargets.length > 1}
                  error={errors.storageTargets?.[target.formId]}
                  index={index}
                  removeTarget={removeStorageTarget}
                  target={target}
                  updateTarget={updateTarget}
                />
              ))}
            </div>
          </div>

          <div className="mt-6 space-y-4">
            <h3 className="text-sm font-semibold">{pageT("sections.segmentBindings", "Segment bindings")}</h3>
            <div className="grid gap-4 lg:grid-cols-2">
              {editableStorageSegmentOrder.map((segmentKey) => {
                const segmentMeta = segmentBindingMeta.find((entry) => entry.key === segmentKey);
                if (!segmentMeta) return null;
                return (
                  <Field
                    key={segmentMeta.key}
                    label={pageT(`segmentBindings.${segmentMeta.key}`, "{{label}} target", { label: pageT(`segments.${segmentMeta.key}`, segmentMeta.label) })}
                    error={errors.segmentBindings?.[segmentMeta.key]}
                    hint={hintT("segmentBinding")}
                  >
                    <SelectInput
                      aria-label={pageT(`segmentBindings.${segmentMeta.key}`, "{{label}} target", { label: pageT(`segments.${segmentMeta.key}`, segmentMeta.label) })}
                      className="w-full"
                      value={form.segmentBindings[segmentMeta.key]}
                      onChange={(event) => updateSegmentBinding(segmentMeta.key, event.target.value)}
                    >
                      <option value="">{pageT("storageTargets.select", "Select storage target")}</option>
                      {form.storageTargets.map((target, targetIndex) => (
                        <option key={target.formId} value={target.id.trim()}>
                          {ragStorageTargetOptionLabel(target, targetIndex)}
                        </option>
                      ))}
                    </SelectInput>
                  </Field>
                );
              })}
            </div>
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold">{pageT("sections.capabilities", "Capabilities")}</h2>
          </div>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {[
              { key: "fulltextEnabled", label: pageT("capabilities.fulltext", "Enable fulltext index") },
              { key: "vectorEnabled", label: pageT("capabilities.vector", "Enable vector index") },
              { key: "treeEnabled", label: pageT("capabilities.tree", "Enable summary tree") },
              { key: "graphEnabled", label: pageT("capabilities.graph", "Enable graph capability"), disabled: !graphCapabilityAvailable },
              { key: "reservedEnabled", label: pageT("capabilities.reserved", "Reserved corpus") },
            ].map(({ key, label, disabled }) => (
              <label
                key={key}
                className={`flex h-10 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] px-3 text-sm ${
                  disabled ? "cursor-not-allowed opacity-60" : ""
                }`}
              >
                <input
                  aria-label={label}
                  disabled={disabled}
                  type="checkbox"
                  checked={Boolean(form[key as keyof RagCorpusFormState])}
                  onChange={(event) => updateForm(key as keyof RagCorpusFormState, event.target.checked as never)}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold">{pageT("sections.fieldMapping", "Field mapping")}</h2>
          </div>
          <div className="space-y-5">
            <FieldMappingSection
              addButtonLabel={pageT("mapping.addSource", "Add source mapping")}
              contractRows={sourceContractMappings}
              customRows={sourceCustomMappings}
              error={errors.sourceMappings}
              mappingKey="sourceMappings"
              onAdd={() => addMapping("sourceMappings")}
              onRemove={(index) => removeMapping("sourceMappings", index)}
              onToggleContractFields={() => setShowSourceContractMappings((current) => !current)}
              onUpdate={(index, patch) => updateMapping("sourceMappings", index, patch)}
              showContractRows={showSourceContractMappings}
              title={pageT("mapping.sourceMapping", "Source mapping")}
              toggleContractLabel={showSourceContractMappings ? pageT("mapping.hideSourceContract", "Hide source contract fields") : pageT("mapping.showSourceContract", "Show source contract fields")}
            />
            <FieldMappingSection
              addButtonLabel={pageT("mapping.addTextUnit", "Add text unit mapping")}
              contractRows={textUnitContractMappings}
              customRows={textUnitCustomMappings}
              error={errors.textUnitMappings}
              mappingKey="textUnitMappings"
              onAdd={() => addMapping("textUnitMappings")}
              onRemove={(index) => removeMapping("textUnitMappings", index)}
              onToggleContractFields={() => setShowTextUnitContractMappings((current) => !current)}
              onUpdate={(index, patch) => updateMapping("textUnitMappings", index, patch)}
              showContractRows={showTextUnitContractMappings}
              title={pageT("mapping.textUnitMapping", "Text unit mapping")}
              toggleContractLabel={showTextUnitContractMappings ? pageT("mapping.hideTextUnitContract", "Hide text unit contract fields") : pageT("mapping.showTextUnitContract", "Show text unit contract fields")}
            />
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="mb-5 flex items-center justify-between gap-3">
            <h2 className="text-base font-semibold">{pageT("sections.filterSchema", "Filter schema")}</h2>
            <Button type="button" variant="secondary" onClick={addFilterField}>{pageT("actions.addFilterField", "Add filter field")}</Button>
          </div>
          <div className="space-y-3">
            {form.filterFields.map((row, index) => (
              <div key={row.id} className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
                <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto] lg:items-end">
                  <Field label={pageT("fields.name", "Name")} hint={hintT("filterName")}>
                    <SelectInput
                      aria-label={pageT("filters.nameAria", "Filter field {{index}} name", { index: index + 1 })}
                      className="w-full"
                      value={row.name}
                      onChange={(event) => updateFilterField(index, { name: event.target.value })}
                    >
                      <option value="">{filterFieldOptions.length ? pageT("filters.selectMappedField", "Select mapped field") : pageT("filters.noMappedFields", "No mapped fields available")}</option>
                      {row.name && !filterFieldOptions.some((option) => option.logicalName === row.name) ? (
                        <option value={row.name}>{pageT("filters.missingMapping", "{{name}} (missing mapping)", { name: row.name })}</option>
                      ) : null}
                      {filterFieldOptions.map((option) => (
                        <option key={option.logicalName} value={option.logicalName}>{option.logicalName}</option>
                      ))}
                    </SelectInput>
                  </Field>
                  <Field label={pageT("fields.type", "Type")} hint={hintT("filterType")}>
                    <SelectInput
                      aria-label={pageT("filters.typeAria", "Filter field {{index}} type", { index: index + 1 })}
                      className="w-full"
                      value={row.type}
                      onChange={(event) => updateFilterField(index, { type: event.target.value as RagFilterFieldFormRow["type"] })}
                    >
                      <option value="string">{pageT("filterTypes.string", "String")}</option>
                      <option value="integer">{pageT("filterTypes.integer", "Integer")}</option>
                      <option value="boolean">{pageT("filterTypes.boolean", "Boolean")}</option>
                    </SelectInput>
                  </Field>
                  <Field label={pageT("fields.operators", "Operators")} hint={hintT("filterOperators")}>
                    <TextInput
                      aria-label={pageT("filters.operatorsAria", "Filter field {{index}} operators", { index: index + 1 })}
                      className="w-full font-mono"
                      value={row.operatorsInput}
                      onChange={(event) => updateFilterField(index, { operatorsInput: event.target.value })}
                    />
                  </Field>
                  <div className="flex items-center justify-end lg:pb-1">
                    <Button
                      aria-label={pageT("filters.removeAria", "Remove filter field {{index}}", { index: index + 1 })}
                      size="icon"
                      type="button"
                      variant="ghost"
                      onClick={() => removeFilterField(index)}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                    </Button>
                  </div>
                </div>
              </div>
            ))}
            {errors.filterFields ? <p className="text-[12px] text-[var(--color-error)]">{errors.filterFields}</p> : null}
          </div>
        </SurfaceCard>

        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold">{pageT("sections.payloadPreview", "Backend payload preview")}</h2>
          </div>
          <pre className="max-h-[420px] overflow-auto rounded-lg bg-[var(--color-surface-container)] p-4 text-xs leading-5 text-[var(--color-on-surface)]">
            {JSON.stringify(payloadPreview, null, 2)}
          </pre>
        </SurfaceCard>
      </form>
    </>
  );
}
