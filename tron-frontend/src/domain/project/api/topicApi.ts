import { requestApi } from "@/shared/api/httpClient";
import type { DirectoryRecord } from "@/domain/directory/api/directoryApi";

export type TopicReferenceKind = "Directory" | "Repository" | "Resource" | "Task" | "Link";
export interface TopicReference {
  referenceCode: string;
  kind: TopicReferenceKind;
  targetCode: string;
  connectionCode: string | null;
  label: string;
  url: string | null;
  available: boolean;
  directoryPath: string | null;
}
export interface TopicMemo {
  projectCode: string;
  notes: string;
  revision: number;
  references: TopicReference[];
}
export interface AddTopicReference {
  revision: number;
  kind: TopicReferenceKind;
  targetCode: string;
  connectionCode: string | null;
  label: string;
  url: string | null;
}

export function getTopicMemo(projectCode: string, signal?: AbortSignal) {
  return requestApi<TopicMemo>("GET /api/v1/topics/{projectCode}/memo", { pathParams: { projectCode }, signal, cache: "no-store" });
}
export function saveTopicNotes(projectCode: string, revision: number, notes: string) {
  const body = { revision, notes };
  return requestApi<TopicMemo>("PUT /api/v1/topics/{projectCode}/memo", {
    pathParams: { projectCode }, body, retryOnUnauthorized: false,
    keepalive: new TextEncoder().encode(JSON.stringify(body)).length < 60_000,
  });
}
export function addTopicReference(projectCode: string, input: AddTopicReference) {
  return requestApi<TopicMemo>("POST /api/v1/topics/{projectCode}/references", { pathParams: { projectCode }, body: input });
}
export function removeTopicReference(projectCode: string, referenceCode: string, revision: number) {
  return requestApi<TopicMemo>("DELETE /api/v1/topics/{projectCode}/references/{referenceCode}", { pathParams: { projectCode, referenceCode }, queryParams: { revision } });
}

export interface CheckoutOptions {
  providerId: string;
  suggestedPath: string;
  existing: DirectoryRecord[];
  cloneSupported: boolean;
}
export function getCheckoutOptions(projectCode: string, referenceCode: string, signal?: AbortSignal) {
  return requestApi<CheckoutOptions>("GET /api/v1/topics/{projectCode}/references/{referenceCode}/checkout", { pathParams: { projectCode, referenceCode }, signal });
}
export function prepareCheckout(projectCode: string, referenceCode: string, input: { mode: "Existing"; path: string } | { mode: "Clone"; path: string; username: string }) {
  return requestApi<TopicMemo>("POST /api/v1/topics/{projectCode}/references/{referenceCode}/checkout", { pathParams: { projectCode, referenceCode }, body: input, retryOnUnauthorized: false });
}
