import {
  createContext,
  useContext,
  useLayoutEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  applySystemFontSize,
  defaultSystemFontSize,
  isSystemFontSize,
  readStoredSystemFontSize,
  systemFontSizeStorageKey,
} from "@/shared/lib/systemFontSize";

export type AppTheme = "light" | "matrix-dark";

interface ThemeOption {
  labelKey: string;
  value: AppTheme;
}

interface ThemeContextValue {
  fontSize: number;
  setFontSize: (fontSize: number) => void;
  setTheme: (theme: AppTheme) => void;
  theme: AppTheme;
  themes: readonly ThemeOption[];
}

const themeStorageKey = "tron.theme";

const themeOptions = [
  { value: "light", labelKey: "shell:theme.options.light" },
  { value: "matrix-dark", labelKey: "shell:theme.options.matrixDark" },
] as const satisfies readonly ThemeOption[];

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

function isAppTheme(value: string): value is AppTheme {
  return value === "light" || value === "matrix-dark";
}

function getStoredTheme(): AppTheme | undefined {
  if (typeof window === "undefined") return undefined;
  const storedTheme = window.localStorage.getItem(themeStorageKey);
  return storedTheme && isAppTheme(storedTheme) ? storedTheme : undefined;
}

function getStoredFontSize(): number | undefined {
  if (typeof window === "undefined") return undefined;
  return readStoredSystemFontSize(window.localStorage);
}

function applyTheme(theme: AppTheme) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.style.colorScheme = theme === "light" ? "light" : "dark";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<AppTheme>(() => getStoredTheme() ?? "light");
  const [fontSize, setFontSize] = useState<number>(() => getStoredFontSize() ?? defaultSystemFontSize);

  useLayoutEffect(() => {
    applyTheme(theme);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(themeStorageKey, theme);
    }
  }, [theme]);

  useLayoutEffect(() => {
    applySystemFontSize(fontSize);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(systemFontSizeStorageKey, String(fontSize));
    }
  }, [fontSize]);

  function updateFontSize(nextFontSize: number) {
    if (isSystemFontSize(nextFontSize)) {
      setFontSize(nextFontSize);
    }
  }

  const contextValue = useMemo<ThemeContextValue>(() => ({
    fontSize,
    setFontSize: updateFontSize,
    setTheme,
    theme,
    themes: themeOptions,
  }), [fontSize, theme]);

  return <ThemeContext.Provider value={contextValue}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const context = useContext(ThemeContext);

  if (!context) {
    throw new Error("useTheme must be used within ThemeProvider.");
  }

  return context;
}
