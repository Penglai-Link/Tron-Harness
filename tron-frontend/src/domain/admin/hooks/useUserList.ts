import { listUsers } from "@/domain/admin/mock/data";
import {
  selectPagedRecords,
  useServiceQuery,
} from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";
import type { UserRecord } from "@/domain/admin/model";

export function useUserList(): UserRecord[] {
  return useServiceQuery<UserRecord[]>({
    domain: "admin",
    endpoint: adminApiEndpoints.listUsers,
    mockData: () => listUsers(),
    select: selectPagedRecords<UserRecord>,
    deps: [],
  });
}
