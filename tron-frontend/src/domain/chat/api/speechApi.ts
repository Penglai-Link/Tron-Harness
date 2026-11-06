import { getApiAccessToken, requestApi } from "@/shared/api/httpClient";

export async function transcribeAudio(audio: Blob, signal: AbortSignal) {
  signal.throwIfAborted();
  // Pin the account before asynchronous encoding; never cancel under a new login.
  const token = getApiAccessToken();
  const headers = token ? { Authorization: `Bearer ${token}` } : undefined;
  const audioBase64 = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error);
    reader.onload = () => resolve(String(reader.result).split(",", 2)[1]);
    reader.readAsDataURL(audio);
  });
  signal.throwIfAborted();
  const requestId = crypto.randomUUID();
  const cancel = () => {
    // This request must survive aborting the upload and ordinary page unload.
    void requestApi<{ cancelled: boolean }>("DELETE /api/v1/models/speech-transcriptions/{requestId}", {
      pathParams: { requestId }, headers, keepalive: true, retryOnUnauthorized: false,
    }).catch(error => console.warn("Could not cancel speech input on the server", { requestId, status: error?.status }));
  };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    return await requestApi<{ text: string }>("POST /api/v1/models/speech-transcriptions", {
      body: { requestId, audioBase64, mimeType: audio.type }, signal, headers, retryOnUnauthorized: false,
    });
  } finally {
    signal.removeEventListener("abort", cancel);
  }
}
