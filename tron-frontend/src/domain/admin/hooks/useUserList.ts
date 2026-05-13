import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchAdminUserPage,
  type AdminPage,
} from "@/domain/admin/api";
import { listUsers } from "@/domain/admin/mock/data";
import type {
  UserRecord,
  UserStatus,
} from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

interface UseUserListOptions {
  current?: number;
  search?: string;
  size?: number;
  status?: UserStatus | "" | "All";
}

export interface UserListResult extends AdminPage<UserRecord> {
  loading: boolean;
  refresh: () => void;
  upsertUser: (user: UserRecord) => void;
}

function userMatchesSearch(user: UserRecord, search: string) {
  if (!search.trim()) return true;
  const searchable = [
    user.userCode,
    user.username,
    user.displayName,
    user.firstName,
    user.lastName,
    user.email,
    user.status,
    ...user.roleCodes,
    ...user.spaceCodes,
  ].join(" ").toLowerCase();

  return searchable.includes(search.trim().toLowerCase());
}

function makeMockPage(users: UserRecord[], options: Required<Pick<UseUserListOptions, "current" | "search" | "size">> & Pick<UseUserListOptions, "status">): AdminPage<UserRecord> {
  const filtered = users.filter((user) => {
    const statusMatches = !options.status || options.status === "All" || user.status === options.status;
    return statusMatches && userMatchesSearch(user, options.search);
  });
  const current = Math.max(1, options.current);
  const start = (current - 1) * options.size;

  return {
    current,
    size: options.size,
    total: filtered.length,
    records: filtered.slice(start, start + options.size),
  };
}

function upsertRecord(records: UserRecord[], user: UserRecord) {
  const index = records.findIndex((item) => item.userCode === user.userCode);
  if (index === -1) return [user, ...records];
  return records.map((item) => (item.userCode === user.userCode ? user : item));
}

export function useUserList(options: UseUserListOptions = {}): UserListResult {
  const dataSource = getServiceDataSource("admin");
  const current = options.current ?? 1;
  const search = options.search ?? "";
  const size = options.size ?? 15;
  const status = options.status === "All" ? "All" : options.status || "Active";
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const mockPage = useMemo(
    () => makeMockPage(listUsers(), { current, search, size, status }),
    [current, search, size, status],
  );
  const [page, setPage] = useState<AdminPage<UserRecord>>(mockPage);

  useEffect(() => {
    if (dataSource === "api") return;
    setPage(mockPage);
  }, [dataSource, mockPage]);

  useEffect(() => {
    if (dataSource !== "api") return;

    const controller = new AbortController();
    setLoading(true);

    fetchAdminUserPage({ current, search, size, status }, controller.signal)
      .then((nextPage) => {
        setPage(nextPage);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[admin] Falling back to mock user data.", error);
        setPage(mockPage);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [current, dataSource, mockPage, reloadKey, search, size, status]);

  const refresh = useCallback(() => {
    setReloadKey((value) => value + 1);
  }, []);

  const upsertUser = useCallback((user: UserRecord) => {
    setPage((currentPage) => {
      const exists = currentPage.records.some((item) => item.userCode === user.userCode);
      return {
        ...currentPage,
        records: upsertRecord(currentPage.records, user),
        total: exists ? currentPage.total : currentPage.total + 1,
      };
    });
  }, []);

  return {
    ...page,
    loading,
    refresh,
    upsertUser,
  };
}
