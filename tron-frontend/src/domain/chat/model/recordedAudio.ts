/** Normalize browser recordings to mono PCM WAV for all transcription providers. */
export async function recordingToWav(blob: Blob): Promise<Blob> {
  const decoder = new OfflineAudioContext(1, 1, 16000);
  const decoded = await decoder.decodeAudioData(await blob.arrayBuffer());
  if (!decoded.length || decoded.duration > 65) throw new Error("invalid recording duration");
  const context = new OfflineAudioContext(1, Math.ceil(decoded.duration * 16000), 16000);
  const source = context.createBufferSource();
  source.buffer = decoded;
  source.connect(context.destination);
  source.start();
  const audio = (await context.startRendering()).getChannelData(0);
  const bytes = new ArrayBuffer(44 + audio.length * 2);
  const view = new DataView(bytes);
  const text = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) view.setUint8(offset + i, value.charCodeAt(i));
  };
  text(0, "RIFF"); view.setUint32(4, bytes.byteLength - 8, true); text(8, "WAVE");
  text(12, "fmt "); view.setUint32(16, 16, true); view.setUint16(20, 1, true);
  view.setUint16(22, 1, true); view.setUint32(24, 16000, true); view.setUint32(28, 32000, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true); text(36, "data");
  view.setUint32(40, audio.length * 2, true);
  audio.forEach((sample, i) => view.setInt16(44 + i * 2, Math.max(-1, Math.min(1, sample)) * (sample < 0 ? 32768 : 32767), true));
  return new Blob([bytes], { type: "audio/wav" });
}
