import { requestApi } from "@/shared/api/httpClient";
export const checkinApi = {
  status: () => requestApi<{ todayCheckedIn: boolean | null; todayRewardPoints: number | null }>("GET /api/v1/account/checkin"),
  checkin: () => requestApi<{ points: number | null; continuousDays: number | null }>("POST /api/v1/account/checkin", { retryOnUnauthorized: false }),
};
