import { describe, expect, it } from "vitest";

import {
  buildOpenApiCanonicalRequest,
  buildOpenApiSignedHeaders,
  computeOpenApiBodyHashHex,
  randomHex,
} from "./openapi-signing";

describe("openapi signing helpers", () => {
  it("builds canonical requests with normalized query order", async () => {
    const bodyHashHex = await computeOpenApiBodyHashHex("{\"workspaceCode\":\"TRON_CORE\",\"title\":\"Hello\"}");
    const canonical = buildOpenApiCanonicalRequest({
      openAppCode: "OPENAPP-TEST-0001",
      bodyHashHex,
      method: "POST",
      nonce: "abc123nonce789",
      path: "/open-api/v1/chats/sessions",
      rawQuery: "z=last&a=first&a=again",
      timestamp: 1718100000,
      userCode: "USR-0001",
    });

    expect(canonical).toBe(
      "POST\n/open-api/v1/chats/sessions\na=again&a=first&z=last\nd5f7daf53bc73b9d7b6a8ec638f713d1d5f17072e296c3ad29d660d5562cd2b0\nOPENAPP-TEST-0001\nUSR-0001\n1718100000\nabc123nonce789",
    );
  });

  it("builds deterministic signed headers for the formal HMAC contract", async () => {
    const headers = await buildOpenApiSignedHeaders({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      body: "{\"workspaceCode\":\"TRON_CORE\",\"title\":\"Hello\"}",
      method: "POST",
      nonce: "abc123nonce789",
      path: "/open-api/v1/chats/sessions",
      rawQuery: "z=last&a=first&a=again",
      timestamp: 1718100000,
      userCode: "USR-0001",
    });

    expect(headers).toEqual({
      "X-Tron-App-Code": "OPENAPP-TEST-0001",
      "X-Tron-Nonce": "abc123nonce789",
      "X-Tron-Signature": "1ad2293472e537bbc46e8cdc694c923e7f373353e8f200141cbec53433b85b00",
      "X-Tron-Signature-Version": "TRON-HMAC-SHA256-V1",
      "X-Tron-Timestamp": "1718100000",
      "X-Tron-User-Code": "USR-0001",
    });
  });

  it("generates fixed-length hexadecimal nonces", () => {
    const nonce = randomHex(12);

    expect(nonce).toMatch(/^[a-f0-9]{12}$/);
  });
});
