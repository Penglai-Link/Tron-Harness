export type BackgroundEffect = "off" | "particles" | "snowfall" | "stars" | "aurora";
export type BackgroundImageFit = "cover" | "contain" | "fill";
export type BackgroundImagePosition = "center" | "top" | "bottom";

export interface BackgroundEffectSettings {
  effect: BackgroundEffect;
  count: number;
  size: number;
  color: string;
  opacity: number;
  speed: number;
}

export interface BackgroundImageSettings {
  fileName: string | null;
  fit: BackgroundImageFit;
  position: BackgroundImagePosition;
  opacity: number;
  blur: number;
}

const effectSettingsKey = "tron.background-effect-settings";
const imageSettingsKey = "tron.background-image-settings";
const imageDatabaseName = "tron-appearance";
const imageStoreName = "background";
const imageRecordKey = "custom-image";

export const defaultBackgroundEffectSettings: BackgroundEffectSettings = {
  effect: "off",
  count: 150,
  size: 2.5,
  color: "#2f5f9f",
  opacity: 0.85,
  speed: 1,
};

export const defaultBackgroundImageSettings: BackgroundImageSettings = {
  fileName: null,
  fit: "cover",
  position: "center",
  opacity: 0.8,
  blur: 0,
};

const effects: BackgroundEffect[] = ["off", "particles", "snowfall", "stars", "aurora"];
const fits: BackgroundImageFit[] = ["cover", "contain", "fill"];
const positions: BackgroundImagePosition[] = ["center", "top", "bottom"];

function clamp(value: unknown, min: number, max: number, fallback: number) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(max, Math.max(min, value))
    : fallback;
}

export function normalizeBackgroundEffectSettings(value: unknown): BackgroundEffectSettings {
  const stored = typeof value === "object" && value !== null
    ? value as Partial<BackgroundEffectSettings>
    : {};
  return {
    effect: effects.includes(stored.effect as BackgroundEffect)
      ? stored.effect as BackgroundEffect
      : defaultBackgroundEffectSettings.effect,
    count: Math.round(clamp(stored.count, 20, 400, defaultBackgroundEffectSettings.count)),
    size: clamp(stored.size, 0.5, 8, defaultBackgroundEffectSettings.size),
    color: typeof stored.color === "string" && /^#[\da-f]{6}$/i.test(stored.color)
      ? stored.color
      : defaultBackgroundEffectSettings.color,
    opacity: clamp(stored.opacity, 0.05, 1, defaultBackgroundEffectSettings.opacity),
    speed: clamp(stored.speed, 0.1, 3, defaultBackgroundEffectSettings.speed),
  };
}

export function readBackgroundEffectSettings(storage: Storage): BackgroundEffectSettings {
  try {
    const raw = JSON.parse(storage.getItem(effectSettingsKey) ?? "null") as Partial<BackgroundEffectSettings> | null;
    const settings = normalizeBackgroundEffectSettings(raw);
    // Earlier builds reused Matrix's faint particle defaults for every effect.
    // Give those saved non-particle selections visible, effect-appropriate defaults.
    if (raw?.effect === "snowfall" || raw?.effect === "stars" || raw?.effect === "aurora") {
      const color = raw.color === "#83e8a0" ? defaultBackgroundEffectSettings.color : settings.color;
      const opacity = raw.opacity === 0.55 ? 0.9 : settings.opacity;
      const size = raw.size === 1.25
        ? raw.effect === "snowfall" ? 3 : raw.effect === "stars" ? 2 : settings.size
        : settings.size;
      return { ...settings, color, opacity, size };
    }
    return settings;
  } catch {
    return defaultBackgroundEffectSettings;
  }
}

export function hasStoredBackgroundEffectSettings(storage: Storage) {
  return storage.getItem(effectSettingsKey) !== null;
}

export function writeBackgroundEffectSettings(storage: Storage, settings: BackgroundEffectSettings) {
  storage.setItem(effectSettingsKey, JSON.stringify(normalizeBackgroundEffectSettings(settings)));
}

export function normalizeBackgroundImageSettings(value: unknown): BackgroundImageSettings {
  const stored = typeof value === "object" && value !== null
    ? value as Partial<BackgroundImageSettings>
    : {};
  return {
    fileName: typeof stored.fileName === "string" ? stored.fileName.slice(0, 255) : null,
    fit: fits.includes(stored.fit as BackgroundImageFit)
      ? stored.fit as BackgroundImageFit
      : defaultBackgroundImageSettings.fit,
    position: positions.includes(stored.position as BackgroundImagePosition)
      ? stored.position as BackgroundImagePosition
      : defaultBackgroundImageSettings.position,
    opacity: clamp(stored.opacity, 0.1, 1, defaultBackgroundImageSettings.opacity),
    blur: clamp(stored.blur, 0, 24, defaultBackgroundImageSettings.blur),
  };
}

export function readBackgroundImageSettings(storage: Storage): BackgroundImageSettings {
  try {
    return normalizeBackgroundImageSettings(JSON.parse(storage.getItem(imageSettingsKey) ?? "null"));
  } catch {
    return defaultBackgroundImageSettings;
  }
}

export function writeBackgroundImageSettings(storage: Storage, settings: BackgroundImageSettings) {
  storage.setItem(imageSettingsKey, JSON.stringify(normalizeBackgroundImageSettings(settings)));
}

function openImageDatabase(): Promise<IDBDatabase> {
  if (typeof indexedDB === "undefined") return Promise.reject(new Error("Image storage is unavailable"));
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(imageDatabaseName, 1);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(imageStoreName)) {
        request.result.createObjectStore(imageStoreName);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("Could not open image storage"));
  });
}

export async function readBackgroundImage(): Promise<Blob | null> {
  const database = await openImageDatabase();
  try {
    return await new Promise((resolve, reject) => {
      const request = database.transaction(imageStoreName, "readonly")
        .objectStore(imageStoreName).get(imageRecordKey) as IDBRequest<Blob | undefined>;
      request.onsuccess = () => resolve(request.result ?? null);
      request.onerror = () => reject(request.error ?? new Error("Could not read background image"));
    });
  } finally {
    database.close();
  }
}

export async function saveBackgroundImage(image: Blob): Promise<void> {
  const database = await openImageDatabase();
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(imageStoreName, "readwrite");
      transaction.objectStore(imageStoreName).put(image, imageRecordKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error ?? new Error("Could not save background image"));
      transaction.onabort = () => reject(transaction.error ?? new Error("Could not save background image"));
    });
  } finally {
    database.close();
  }
}

export async function deleteBackgroundImage(): Promise<void> {
  const database = await openImageDatabase();
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(imageStoreName, "readwrite");
      transaction.objectStore(imageStoreName).delete(imageRecordKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error ?? new Error("Could not remove background image"));
      transaction.onabort = () => reject(transaction.error ?? new Error("Could not remove background image"));
    });
  } finally {
    database.close();
  }
}
