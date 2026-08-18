function randomSuffix() {
  const uuid = globalThis.crypto?.randomUUID?.();
  if (uuid) return uuid.replaceAll("-", "").toUpperCase();

  return `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`.toUpperCase();
}

export function generateConnectionResourceCode(prefix: string) {
  return `${prefix}_${randomSuffix()}`;
}
