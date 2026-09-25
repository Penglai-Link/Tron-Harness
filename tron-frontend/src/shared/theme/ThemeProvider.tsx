import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { applySystemFontSize, defaultSystemFontSize, isSystemFontSize, readStoredSystemFontSize, systemFontSizeStorageKey } from "@/shared/lib/systemFontSize";
import { useAuthStore } from "@/store/authStore";
import { skinApi } from "./skinApi";
import { prepareSkin, type PreparedSkin } from "./skinEngine";
import type { Skin } from "./Skin";
import light from "./builtin/light.json";
import matrix from "./builtin/matrix-dark.json";

export type AppTheme = "light" | "matrix-dark";
const builtins = [light, matrix] as Skin[];
interface ThemeContextValue {
  fontSize: number;
  setFontSize: (size: number) => void;
  theme: AppTheme;
  activeSkin: Skin;
  skins: Skin[];
  loading: boolean;
  error: string | null;
  refreshSkins: () => Promise<void>;
  selectSkin: (id: string) => Promise<void>;
  importSkin: (file: File) => Promise<void>;
  uninstallSkin: (id: string) => Promise<void>;
  previewSkin: (id: string) => Promise<void>;
  stopPreviewSkin: () => void;
  previewSkinId: string | null;
}
const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const status = useAuthStore((state) => state.status);
  const [activeSkin, setActiveSkin] = useState<Skin>(() => localStorage.getItem("tron.theme") === "matrix-dark" ? builtins[1] : builtins[0]);
  const [skins, setSkins] = useState<Skin[]>(builtins);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [previewSkinId, setPreviewSkinId] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState(() => readStoredSystemFontSize(localStorage) ?? defaultSystemFontSize);
  const applied = useRef<PreparedSkin | null>(null);
  const activeSignature = useRef<string | null>(null);
  const generation = useRef(0);
  const mutation = useRef(false);
  const previewPrepared = useRef<PreparedSkin | null>(null);
  const previewGeneration = useRef(0);

  const stopPreviewSkin = useCallback(() => {
    previewGeneration.current++;
    previewPrepared.current?.dispose();
    previewPrepared.current = null;
    setPreviewSkinId(null);
    applied.current?.apply();
  }, []);

  const commit = useCallback((skin: Skin, prepared: PreparedSkin) => {
    applied.current?.dispose();
    prepared.apply();
    applied.current = prepared;
    activeSignature.current = JSON.stringify(skin);
    setActiveSkin(skin);
    localStorage.setItem("tron.theme", skin.mode === "Dark" ? "matrix-dark" : "light");
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = activeSkin.mode === "Dark" ? "matrix-dark" : "light";
  }, [activeSkin.mode]);

  const refreshSkins = useCallback(async () => {
    if (mutation.current) return;
    const request = ++generation.current;
    let prepared: PreparedSkin | undefined;
    try {
      const [list, { skin }] = await Promise.all([skinApi.list(), skinApi.active()]);
      if (request !== generation.current) return;
      // Keep recovery choices available even when the active skin's assets fail.
      setSkins(list);
      if (JSON.stringify(skin) !== activeSignature.current) prepared = await prepareSkin(skin);
      if (request !== generation.current) { prepared?.dispose(); return; }
      if (prepared) { stopPreviewSkin(); commit(skin, prepared); }
      setError(null);
    } catch (reason) {
      if (request === generation.current) setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [commit, stopPreviewSkin]);

  useEffect(() => {
    if (status !== "authenticated") { setLoading(false); return; }
    setLoading(true);
    void refreshSkins();
    const stop = skinApi.watch(() => void refreshSkins());
    const onFocus = () => void refreshSkins();
    window.addEventListener("focus", onFocus);
    return () => { generation.current++; stop(); window.removeEventListener("focus", onFocus); };
  }, [status, refreshSkins]);
  useEffect(() => () => { generation.current++; previewPrepared.current?.dispose(); applied.current?.dispose(); }, []);

  async function selectSkin(id: string) {
    if (mutation.current) return;
    const skin = skins.find((item) => item.id === id);
    if (!skin) return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    let prepared: PreparedSkin | undefined;
    try {
      prepared = await prepareSkin(skin);
      if (request !== generation.current) return;
      await skinApi.activate(id);
      if (request !== generation.current) return;
      commit(skin, prepared);
      prepared = undefined;
      setError(null);
    } finally {
      prepared?.dispose();
      mutation.current = false;
      setLoading(false);
      if (request === generation.current) void refreshSkins();
    }
  }
  async function importSkin(file: File) {
    if (mutation.current) return;
    mutation.current = true;
    stopPreviewSkin();
    generation.current++;
    setLoading(true);
    try { await skinApi.install(file); }
    finally { mutation.current = false; setLoading(false); }
    await refreshSkins();
  }
  async function uninstallSkin(id: string) {
    if (mutation.current || id === "light" || id === "matrix-dark") return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    let fallback: PreparedSkin | undefined;
    try {
      if (activeSkin.id === id) fallback = await prepareSkin(builtins[0]);
      await skinApi.uninstall(id);
      if (request !== generation.current) return;
      if (fallback) {
        commit(builtins[0], fallback);
        fallback = undefined;
      }
      setSkins((current) => current.filter((skin) => skin.id !== id));
      setError(null);
    } finally {
      fallback?.dispose();
      mutation.current = false;
      setLoading(false);
      if (request === generation.current) void refreshSkins();
    }
  }
  async function previewSkin(id: string) {
    const skin = skins.find((item) => item.id === id);
    if (!skin) return;
    stopPreviewSkin();
    const request = previewGeneration.current;
    const prepared = await prepareSkin(skin);
    if (request !== previewGeneration.current) { prepared.dispose(); return; }
    prepared.apply();
    previewPrepared.current = prepared;
    setPreviewSkinId(id);
  }
  useLayoutEffect(() => {
    applySystemFontSize(fontSize);
    localStorage.setItem(systemFontSizeStorageKey, String(fontSize));
  }, [fontSize]);

  return <ThemeContext.Provider value={{
    fontSize, setFontSize: (size) => { if (isSystemFontSize(size)) setFontSize(size); },
    theme: activeSkin.mode === "Dark" ? "matrix-dark" : "light", activeSkin, skins,
    loading, error, refreshSkins, selectSkin, importSkin, uninstallSkin, previewSkin, stopPreviewSkin, previewSkinId,
  }}>{children}</ThemeContext.Provider>;
}
export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider.");
  return context;
}
