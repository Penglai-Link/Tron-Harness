export const defaultSystemFontSize = 14;
export const minimumSystemFontSize = 12;
export const maximumSystemFontSize = 20;
export const systemFontSizeStorageKey = "tron.systemFontSize";
export const systemFontSizeCssVariable = "--tron-system-font-size";

interface StorageReader {
  getItem: (key: string) => string | null;
}

interface FontSizeRoot {
  style: {
    fontSize: string;
    setProperty: (name: string, value: string) => void;
  };
}

export function isSystemFontSize(value: unknown): value is number {
  return typeof value === "number"
    && Number.isInteger(value)
    && value >= minimumSystemFontSize
    && value <= maximumSystemFontSize;
}

export function parseSystemFontSize(value: string | null): number | undefined {
  if (value === null || value.trim() === "") return undefined;

  const fontSize = Number(value);
  return isSystemFontSize(fontSize) ? fontSize : undefined;
}

export function readStoredSystemFontSize(storage: StorageReader): number | undefined {
  return parseSystemFontSize(storage.getItem(systemFontSizeStorageKey));
}

export function applySystemFontSize(
  fontSize: number,
  root: FontSizeRoot | undefined = typeof document === "undefined" ? undefined : document.documentElement,
) {
  if (!root) return;
  const value = `${fontSize}px`;
  root.style.fontSize = value;
  root.style.setProperty(systemFontSizeCssVariable, value);
}
