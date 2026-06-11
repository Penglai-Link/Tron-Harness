import type { BodyInitLike } from "./transport";

export const OPEN_API_SIGNATURE_VERSION = "TRON-HMAC-SHA256-V1";

const encoder = new TextEncoder();

export interface OpenApiSigningInput {
  openAppCode: string;
  appKey: string;
  body?: BodyInitLike;
  method: string;
  nonce?: string;
  path: string;
  rawQuery?: string;
  timestamp?: number;
  userCode: string;
}

function bytesToHex(bytes: Uint8Array) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function copyArrayBufferView(view: ArrayBufferView) {
  const bytes = new Uint8Array(view.byteLength);
  bytes.set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
  return bytes;
}

function requireWebCrypto() {
  if (!globalThis.crypto?.subtle) {
    throw new Error("Web Crypto is required for HMAC OpenAPI signing");
  }

  return globalThis.crypto;
}

function normalizedRequired(value: string | undefined, field: string) {
  const normalized = value?.trim() ?? "";
  if (!normalized) {
    throw new Error(`${field} is required for HMAC OpenAPI signing`);
  }
  return normalized;
}

function normalizeQueryString(rawQuery?: string) {
  if (!rawQuery?.trim()) return "";

  const pairs = Array.from(new URLSearchParams(rawQuery).entries()).sort(([leftKey, leftValue], [rightKey, rightValue]) => {
    const keyCompare = leftKey.localeCompare(rightKey);
    return keyCompare !== 0 ? keyCompare : leftValue.localeCompare(rightValue);
  });
  const normalized = new URLSearchParams();
  pairs.forEach(([key, value]) => normalized.append(key, value));
  return normalized.toString();
}

async function resolveBodyBytes(body?: BodyInitLike) {
  if (body === undefined) return new Uint8Array();
  if (typeof body === "string") return encoder.encode(body);
  if (body instanceof URLSearchParams) return encoder.encode(body.toString());
  if (body instanceof Blob) return new Uint8Array(await body.arrayBuffer());
  if (body instanceof ArrayBuffer) return new Uint8Array(body);
  if (ArrayBuffer.isView(body)) return copyArrayBufferView(body);
  if (body instanceof FormData) {
    throw new Error("FormData bodies are not supported by the signed OpenAPI adapter");
  }

  return encoder.encode(JSON.stringify(body));
}

export function buildOpenApiCanonicalRequest(input: {
  openAppCode: string;
  bodyHashHex: string;
  method: string;
  nonce: string;
  path: string;
  rawQuery?: string;
  timestamp: number;
  userCode: string;
}) {
  const method = normalizedRequired(input.method, "method");
  const path = normalizedRequired(input.path, "path");
  const openAppCode = normalizedRequired(input.openAppCode, "openAppCode");
  const userCode = normalizedRequired(input.userCode, "userCode");
  const nonce = normalizedRequired(input.nonce, "nonce");

  return [
    method,
    path,
    normalizeQueryString(input.rawQuery),
    input.bodyHashHex,
    openAppCode,
    userCode,
    String(input.timestamp),
    nonce,
  ].join("\n");
}

export async function computeOpenApiBodyHashHex(body?: BodyInitLike) {
  const crypto = requireWebCrypto();
  const digest = await crypto.subtle.digest("SHA-256", await resolveBodyBytes(body));
  return bytesToHex(new Uint8Array(digest));
}

export async function computeOpenApiSignature(secret: string, canonicalRequest: string) {
  const crypto = requireWebCrypto();
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(normalizedRequired(secret, "appKey")).buffer,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(canonicalRequest).buffer);
  return bytesToHex(new Uint8Array(signature));
}

export function randomHex(length = 12) {
  if (length <= 0) return "";
  const crypto = requireWebCrypto();
  const bytes = new Uint8Array(Math.ceil(length / 2));
  crypto.getRandomValues(bytes);
  return bytesToHex(bytes).slice(0, length);
}

export async function buildOpenApiSignedHeaders(input: OpenApiSigningInput) {
  const openAppCode = normalizedRequired(input.openAppCode, "openAppCode");
  const appKey = normalizedRequired(input.appKey, "appKey");
  const userCode = normalizedRequired(input.userCode, "userCode");
  const timestamp = input.timestamp ?? Math.floor(Date.now() / 1000);
  const nonce = input.nonce ?? randomHex(12);
  const bodyHashHex = await computeOpenApiBodyHashHex(input.body);
  const canonicalRequest = buildOpenApiCanonicalRequest({
    openAppCode,
    bodyHashHex,
    method: input.method,
    nonce,
    path: input.path,
    rawQuery: input.rawQuery,
    timestamp,
    userCode,
  });
  const signature = await computeOpenApiSignature(appKey, canonicalRequest);

  return {
    "X-Tron-App-Code": openAppCode,
    "X-Tron-Nonce": nonce,
    "X-Tron-Signature": signature,
    "X-Tron-Signature-Version": OPEN_API_SIGNATURE_VERSION,
    "X-Tron-Timestamp": String(timestamp),
    "X-Tron-User-Code": userCode,
  };
}
