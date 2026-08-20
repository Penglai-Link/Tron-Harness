import {
  requestApi,
  requestApiBlob,
  type ApiBlobResponse,
} from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import type {
  ChatMessageAttachment,
  ChatSession,
} from "../model";
import { chatApiEndpoints } from "./chatApi";

export interface ChatSessionAttachmentUploadInput {
  file: File;
  kind?: ChatMessageAttachment["kind"];
  metadata?: Record<string, unknown>;
  summary?: string;
}

function mockAttachment(input: ChatSessionAttachmentUploadInput): ChatMessageAttachment {
  const id = `MOCK-CHAT-ATTACH-${Date.now()}`;
  return {
    byteSize: input.file.size,
    contentType: input.file.type || "application/octet-stream",
    fileName: input.file.name,
    id,
    kind: input.kind ?? (input.file.type.startsWith("image/") ? "image" : "file"),
    metadata: input.metadata ?? {},
    resourceCode: id,
    summary: input.summary?.trim() || input.file.name,
    title: input.file.name,
  };
}

export async function uploadChatSessionAttachment(
  session: ChatSession,
  input: ChatSessionAttachmentUploadInput,
): Promise<ChatMessageAttachment> {
  if (getServiceDataSource("chat") === "mock") {
    return mockAttachment(input);
  }

  const body = new FormData();
  if (input.kind) body.set("kind", input.kind);
  if (input.summary?.trim()) body.set("summary", input.summary.trim());
  if (input.metadata && Object.keys(input.metadata).length) {
    body.set("metadata", JSON.stringify(input.metadata));
  }
  body.set("file", input.file, input.file.name);
  return requestApi<ChatMessageAttachment>(chatApiEndpoints.uploadChatSessionAttachment, {
    body,
    pathParams: { sessionCode: session.sessionCode },
  });
}

export function downloadChatSessionAttachment(
  session: ChatSession,
  attachment: ChatMessageAttachment,
): Promise<ApiBlobResponse> {
  const attachmentCode = attachment.resourceCode?.trim() || attachment.id.trim();
  if (!attachmentCode) {
    return Promise.reject(new Error("Chat attachment reference is missing"));
  }
  if (getServiceDataSource("chat") === "mock") {
    return Promise.reject(new Error("Mock chat attachments do not have stored binary content"));
  }
  return requestApiBlob(chatApiEndpoints.downloadChatSessionAttachment, {
    pathParams: {
      attachmentCode,
      sessionCode: session.sessionCode,
    },
  });
}
