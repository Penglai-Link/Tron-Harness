import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
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
import { isUnauthorizedApiError } from "@/shared/api/httpClient";

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
  const [debouncedSearch, setDebouncedSearch] = useState(search);
  const [searchVersion, setSearchVersion] = useState(0);
  const lastSearchRef = useRef(search);
  const skipNextEmptySearchRef = useRef(false);
  const requestSequenceRef = useRef(0);

  const listOptions = useMemo(
    () => ({
      current,
      search: debouncedSearch,
      size,
      status,
    }),
    [current, debouncedSearch, size, status],
  );
  const queryTrimmed = listOptions.search.trim();

  const mockPage = useMemo(
    () => makeMockPage(listUsers(), { current, search: queryTrimmed, size, status }),
    [current, queryTrimmed, size, status],
  );
  const [page, setPage] = useState<AdminPage<UserRecord>>(mockPage);

  useEffect(() => {
    if (dataSource === "api") return;
    setPage(mockPage);
  }, [dataSource, mockPage]);

  useEffect(() => {
    const trimmedSearch = search.trim();
    const previousSearch = lastSearchRef.current.trim();

    if (trimmedSearch === "" && previousSearch !== "") {
      skipNextEmptySearchRef.current = true;
    } else if (trimmedSearch !== "") {
      skipNextEmptySearchRef.current = false;
    }

    lastSearchRef.current = trimmedSearch;

    const timer = window.setTimeout(() => {
      setDebouncedSearch(trimmedSearch);
      if (!(trimmedSearch === "" && debouncedSearch === "")) {
        setSearchVersion((currentVersion) => currentVersion + 1);
      }
    }, 320);

    return () => window.clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    const nonSearchChange = dataSource === "api";
    if (nonSearchChange) {
      skipNextEmptySearchRef.current = false;
    }
  }, [current, dataSource, reloadKey, size, status]);

  useEffect(() => {
    if (dataSource !== "api") return;
    if (queryTrimmed === "" && skipNextEmptySearchRef.current) {
      return;
    }

    const requestId = requestSequenceRef.current + 1;
    requestSequenceRef.current = requestId;
    const controller = new AbortController();
    setLoading(true);

    fetchAdminUserPage({
      current,
      search: queryTrimmed,
      size,
      status,
    }, controller.signal)
      .then((nextPage) => {
        if (requestId !== requestSequenceRef.current || controller.signal.aborted) return;
        setPage(nextPage);
      })
      .catch((error: unknown) => {
        if (requestId !== requestSequenceRef.current || controller.signal.aborted) return;
        if (isUnauthorizedApiError(error)) return;
        console.warn("[admin] Falling back to mock user data.", error);
        setPage(mockPage);
      })
      .finally(() => {
        if (!controller.signal.aborted && requestId === requestSequenceRef.current) setLoading(false);
      });

    return () => controller.abort();
  }, [current, dataSource, mockPage, queryTrimmed, reloadKey, searchVersion, size, status]);

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
