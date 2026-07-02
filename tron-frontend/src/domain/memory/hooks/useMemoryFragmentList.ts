import { useEffect, useMemo, useState } from "react";

import {
  fetchMemoryDomains,
  fetchMemoryRecord,
  fetchMemoryRecords,
  type MemoryWorkDomainRecord,
  type MemoryRecordListQuery,
} from "@/domain/memory/api";
import { useAuthSession } from "@/domain/auth";
import type { MemoryFilterState } from "@/domain/memory/constants";
import type { MemoryRecord, MemorySearchTarget } from "@/domain/memory/model";
import {
  getMemoryFragment,
  getMemoryFragments,
  listMemoryWorkDomainsMock,
} from "@/domain/memory/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { useToast } from "@/shared/ui/Toast";
import { toastManager } from "@/components/ui/toast";

interface UseMemoryFragmentListOptions {
  activeTarget?: MemorySearchTarget;
  filters?: MemoryFilterState;
  query?: string;
  refreshKey?: number;
  workspaceCode?: string;
}

function buildCurrentTargets({
  userCode,
  userLabel,
  workspaceCode,
  workspaceLabel,
}: {
  userCode?: string;
  userLabel?: string;
  workspaceCode?: string;
  workspaceLabel?: string;
}) {
  return [
    {
      code: workspaceCode || "space:current",
      description: "Current space",
      id: "space:current",
      label: workspaceLabel || "Current space",
      resultCount: 0,
      scope: "space" as const,
    },
    {
      code: userCode || "user:current",
      description: "Current user",
      id: "user:current",
      label: userLabel || "Current user",
      resultCount: 0,
      scope: "user" as const,
    },
  ] satisfies MemorySearchTarget[];
}

function buildDomainTargets(domains: MemoryWorkDomainRecord[]) {
  return domains
    .filter((domain) => domain.status === "ACTIVE")
    .map((domain) => ({
      code: domain.domainCode,
      description: "Work domain",
      id: `domain:${domain.domainCode}`,
      label: domain.name,
      resultCount: 0,
      scope: "domain" as const,
    }));
}

export function useMemoryTargets(workspaceCode?: string, refreshKey = 0) {
  const toast = useToast();
  const dataSource = getServiceDataSource("memory");
  const { session } = useAuthSession();
  const {
    selectedPrimarySpace,
    selectedSecondarySpace,
  } = useWorkspaceSpaces();
  const workspaceLabel = selectedSecondarySpace?.name ?? selectedPrimarySpace?.name;
  const userLabel = session?.user.username || session?.user.displayName;
  const userCode = session?.user.userCode;
  const baseTargets = useMemo(() => buildCurrentTargets({
    userCode,
    userLabel,
    workspaceCode,
    workspaceLabel,
  }), [
    userCode,
    userLabel,
    workspaceCode,
    workspaceLabel,
  ]);
  const [targets, setTargets] = useState<MemorySearchTarget[]>(() => baseTargets);

  useEffect(() => {
    if (dataSource === "mock") {
      setTargets([
        ...baseTargets,
        ...buildDomainTargets(listMemoryWorkDomainsMock({ includeDisabled: false })),
      ]);
      return;
    }

    const controller = new AbortController();
    fetchMemoryDomains({ includeDisabled: false }, controller.signal)
      .then((domains) => setTargets([
        ...baseTargets,
        ...buildDomainTargets(domains),
      ]))
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error ? error.message : "Memory targets could not be loaded");
        setTargets(baseTargets);
      });

    return () => controller.abort();
  }, [baseTargets, dataSource, refreshKey, toast]);

  return targets;
}

export function useMemoryFragmentList(options: UseMemoryFragmentListOptions | number = {}) {
  const dataSource = getServiceDataSource("memory");
  const toast = useToast()
  const normalizedOptions = typeof options === "number" ? { refreshKey: options } : options;
  const {
    activeTarget,
    filters,
    query = "",
    refreshKey = 0,
    workspaceCode,
  } = normalizedOptions;
  const [records, setRecords] = useState<MemoryRecord[]>(() => (
    dataSource === "mock" ? getMemoryFragments() : []
  ));

  useEffect(() => {
    if (dataSource === "mock") {
      setRecords(getMemoryFragments());
      return;
    }
    if (!workspaceCode || !activeTarget) {
      setRecords([]);
      return;
    }

    const apiQuery: MemoryRecordListQuery = {
      injectable: filters?.injectable ?? "all",
      memoryStatus: filters?.status ?? "all",
      query,
      reviewStatus: filters?.reviewStatus ?? "all",
      scope: activeTarget.scope,
      scopeCode: activeTarget.code,
    };

    const controller = new AbortController();
    fetchMemoryRecords(workspaceCode, apiQuery, controller.signal)
      .then(setRecords)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error
          ? error.message
          : "Memories could not be loaded")
        setRecords([]);
      });

    return () => controller.abort();
  }, [
    dataSource,
    refreshKey,
    workspaceCode,
    activeTarget?.scope,
    activeTarget?.code,
    filters?.injectable,
    filters?.reviewStatus,
    filters?.status,
    query,
  ]);

  return records;
}

export function useMemoryRecord(workspaceCode: string | undefined, memoryId: string, refreshKey = 0) {
  const toast = useToast();
  const dataSource = getServiceDataSource("memory");
  const [record, setRecord] = useState<MemoryRecord | null>(() => (
    dataSource === "mock" ? getMemoryFragment(memoryId) : null
  ));

  useEffect(() => {
    if (!memoryId) {
      setRecord(null);
      return;
    }
    if (dataSource === "mock") {
      setRecord(getMemoryFragment(memoryId));
      return;
    }
    if (!workspaceCode) {
      setRecord(null);
      return;
    }

    const controller = new AbortController();
    fetchMemoryRecord(workspaceCode, memoryId, controller.signal)
      .then(setRecord)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        toast.error(error instanceof Error ? error.message : "Memory could not be loaded");
        setRecord(null);
      });

    return () => controller.abort();
  }, [dataSource, memoryId, refreshKey, toast, workspaceCode]);

  return record;
}
