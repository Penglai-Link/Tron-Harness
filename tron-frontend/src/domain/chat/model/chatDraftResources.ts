import type { ChatAdapter, ChatAttachmentUploadInput, ChatMessageAttachment, ChatSession } from "@tron/chat-core";

/** Tab-owned files and creation lock survive the draft-to-session transition. */
export class ChatDraftResources {
  private sessionPromise: Promise<ChatSession> | null = null;
  private files = new Map<string, ChatAttachmentUploadInput>();
  private uploads = new Map<string, Promise<ChatMessageAttachment>>();

  createSession(create: () => Promise<ChatSession>) {
    if (!this.sessionPromise) {
      this.sessionPromise = create().catch((error) => {
        this.sessionPromise = null;
        throw error;
      });
    }
    return this.sessionPromise;
  }

  stage(input: ChatAttachmentUploadInput): ChatMessageAttachment {
    const id = `draft-attachment:${crypto.randomUUID()}`;
    this.files.set(id, input);
    return {
      id, kind: input.kind ?? (input.file.type.startsWith("image/") ? "image" : "file"),
      title: input.file.name, fileName: input.file.name, contentType: input.file.type,
      byteSize: input.file.size, metadata: input.metadata ?? {}, summary: input.summary ?? input.file.name,
    };
  }

  file(attachment: ChatMessageAttachment) {
    return this.files.get(attachment.id)?.file;
  }

  private upload(adapter: ChatAdapter, session: ChatSession, attachment: ChatMessageAttachment, signal?: AbortSignal) {
    const file = this.files.get(attachment.id);
    if (!file) return Promise.resolve(attachment);
    const key = `${session.sessionCode}:${attachment.id}`;
    let pending = this.uploads.get(key);
    if (!pending) {
      if (!adapter.uploadAttachment) return Promise.reject(new Error("Chat attachment upload is unavailable"));
      pending = adapter.uploadAttachment(session, file, signal).catch((error) => {
        this.uploads.delete(key);
        throw error;
      });
      this.uploads.set(key, pending);
    }
    return pending;
  }

  adapt(adapter: ChatAdapter): ChatAdapter {
    return {
      ...adapter,
      streamMessage: async (session, input, handlers, signal) => {
        const attachments = await Promise.all((input.attachments ?? []).map((item) => this.upload(adapter, session, item, signal)));
        return adapter.streamMessage(session, { ...input, attachments }, handlers, signal);
      },
      downloadAttachment: async (session, attachment, signal) => {
        const file = this.file(attachment);
        if (file) return file;
        if (!adapter.downloadAttachment) throw new Error("Chat attachment download is unavailable");
        return adapter.downloadAttachment(session, attachment, signal);
      },
    };
  }
}
