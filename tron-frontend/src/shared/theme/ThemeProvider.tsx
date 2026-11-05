import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";
import { applySystemFontSize, defaultSystemFontSize, isSystemFontSize, readStoredSystemFontSize, systemFontSizeStorageKey } from "@/shared/lib/systemFontSize";
import { useAuthStore } from "@/store/authStore";
import { skinApi } from "./skinApi";
import { prepareSkin, type PreparedSkin } from "./skinEngine";
import { revealTheme } from "./themeTransition";
import {
  defaultTypingParticleSettings,
  mountTypingEffect,
  type TypingParticleSettings,
  type TypingParticleShape,
  type TypingParticleStyle,
} from "./typingParticles";
import { resolveSkin, type Skin, type ColorMode } from "./Skin";
import defaultSkin from "./builtin/default.json";
import brandSkin from "./builtin/brand.json";
import { BackgroundLayer } from "./BackgroundLayer";
import {
  deleteBackgroundImage,
  hasStoredBackgroundEffectSettings,
  normalizeBackgroundEffectSettings,
  normalizeBackgroundImageSettings,
  readBackgroundEffectSettings,
  readBackgroundImage,
  readBackgroundImageSettings,
  saveBackgroundImage,
  writeBackgroundEffectSettings,
  writeBackgroundImageSettings,
  type BackgroundEffectSettings,
  type BackgroundImageSettings,
} from "./backgroundPreferences";
import {
  normalizeSidebarAppearanceSettings,
  readSidebarAppearanceSettings,
  writeSidebarAppearanceSettings,
  type SidebarAppearanceSettings,
} from "./sidebarAppearancePreferences";

export type AppTheme = "light" | "matrix-dark";
export type TypingEffect = "off" | "particles" | "glow" | "ripple";
export type AgentReplyEffect = "off" | "typewriter";
const typingEffectStorageKey = "tron.typing-effect";
const typingParticleSettingsStorageKey = "tron.typing-particle-settings";
const agentReplyEffectStorageKey = "tron.agent-reply-effect";
const agentReplySpeedStorageKey = "tron.agent-reply-speed";
function readTypingEffect(): TypingEffect {
  const stored = localStorage.getItem(typingEffectStorageKey);
  return stored === "particles" || stored === "glow" || stored === "ripple" ? stored : "off";
}
function readAgentReplyEffect(): AgentReplyEffect {
  return localStorage.getItem(agentReplyEffectStorageKey) === "off" ? "off" : "typewriter";
}
function readAgentReplySpeed() {
  const raw = localStorage.getItem(agentReplySpeedStorageKey);
  if (raw === null) return 120;
  const stored = Number(raw);
  return Number.isFinite(stored) ? Math.min(240, Math.max(30, stored)) : 120;
}
function readTypingParticleSettings(): TypingParticleSettings {
  try {
    const stored = JSON.parse(localStorage.getItem(typingParticleSettingsStorageKey) ?? "null") as Partial<TypingParticleSettings> | null;
    if (!stored) return defaultTypingParticleSettings;
    const styles: TypingParticleStyle[] = ["colorful", "mosaic", "neon", "custom"];
    const shapes: TypingParticleShape[] = ["circle", "square", "diamond", "star"];
    return {
      style: styles.includes(stored.style as TypingParticleStyle) ? stored.style as TypingParticleStyle : defaultTypingParticleSettings.style,
      shape: shapes.includes(stored.shape as TypingParticleShape) ? stored.shape as TypingParticleShape : defaultTypingParticleSettings.shape,
      count: typeof stored.count === "number" ? Math.min(24, Math.max(4, stored.count)) : defaultTypingParticleSettings.count,
      size: typeof stored.size === "number" ? Math.min(8, Math.max(1, stored.size)) : defaultTypingParticleSettings.size,
      speed: typeof stored.speed === "number" ? Math.min(3, Math.max(0.5, stored.speed)) : defaultTypingParticleSettings.speed,
      color: typeof stored.color === "string" && /^#[\da-f]{6}$/i.test(stored.color)
        ? stored.color
        : defaultTypingParticleSettings.color,
    };
  } catch {
    return defaultTypingParticleSettings;
  }
}
const builtins = [defaultSkin, brandSkin] as Skin[];
const colorModeStorageKey = "tron.color-mode";
function readColorMode(): ColorMode {
  const stored = localStorage.getItem(colorModeStorageKey);
  if (stored === "light" || stored === "dark" || stored === "system") return stored;
  // Preserve an explicitly chosen legacy appearance on upgrade.
  const previous = localStorage.getItem("tron.theme");
  return previous === "light" ? "light" : previous === "matrix-dark" ? "dark" : "system";
}
interface ThemeContextValue {
  fontSize: number;
  setFontSize: (size: number) => void;
  typingEffect: TypingEffect;
  setTypingEffect: (effect: TypingEffect) => void;
  agentReplyEffect: AgentReplyEffect;
  setAgentReplyEffect: (effect: AgentReplyEffect) => void;
  agentReplySpeed: number;
  setAgentReplySpeed: (speed: number) => void;
  typingParticleSettings: TypingParticleSettings;
  setTypingParticleSettings: (settings: TypingParticleSettings) => void;
  backgroundEffectSettings: BackgroundEffectSettings;
  setBackgroundEffectSettings: (settings: BackgroundEffectSettings) => void;
  backgroundImageSettings: BackgroundImageSettings;
  setBackgroundImageSettings: (settings: BackgroundImageSettings) => void;
  sidebarAppearanceSettings: SidebarAppearanceSettings;
  setSidebarAppearanceSettings: (settings: SidebarAppearanceSettings) => void;
  backgroundImageUrl: string | null;
  setBackgroundImage: (image: File) => Promise<void>;
  clearBackgroundImage: () => Promise<void>;
  theme: AppTheme;
  colorMode: ColorMode;
  setColorMode: (mode: ColorMode) => void;
  resolvedMode: "light" | "dark";
  activeSkin: Skin;
  skins: Skin[];
  loading: boolean;
  error: string | null;
  refreshSkins: () => Promise<void>;
  selectSkin: (id: string) => Promise<void>;
  importSkin: (file: File) => Promise<void>;
  installMarketSkin: (id: string) => Promise<void>;
  uninstallSkin: (id: string) => Promise<void>;
  previewSkin: (id: string) => Promise<void>;
  stopPreviewSkin: () => void;
  previewSkinId: string | null;
}
const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const status = useAuthStore((state) => state.status);
  const [activeSkin, setActiveSkin] = useState<Skin>(() => builtins.find((skin) => skin.id === localStorage.getItem("tron.skin")) ?? builtins[0]);
  const [colorMode, setColorModeState] = useState<ColorMode>(readColorMode);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  const resolvedMode = colorMode === "system" ? (systemDark ? "dark" : "light") : colorMode;
  const [displayedSkin, setDisplayedSkin] = useState(activeSkin);
  const [displayedMode, setDisplayedMode] = useState(resolvedMode);
  const [skins, setSkins] = useState<Skin[]>(builtins);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [previewSkinId, setPreviewSkinId] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState(() => readStoredSystemFontSize(localStorage) ?? defaultSystemFontSize);
  const [typingEffect, setTypingEffect] = useState<TypingEffect>(readTypingEffect);
  const [agentReplyEffect, setAgentReplyEffect] = useState<AgentReplyEffect>(readAgentReplyEffect);
  const [agentReplySpeed, setAgentReplySpeed] = useState(readAgentReplySpeed);
  const [typingParticleSettings, setTypingParticleSettings] = useState<TypingParticleSettings>(readTypingParticleSettings);
  const [backgroundEffectSettings, setBackgroundEffectSettings] = useState<BackgroundEffectSettings>(
    () => readBackgroundEffectSettings(localStorage),
  );
  const [backgroundImageSettings, setBackgroundImageSettings] = useState<BackgroundImageSettings>(
    () => readBackgroundImageSettings(localStorage),
  );
  const [sidebarAppearanceSettings, setSidebarAppearanceSettingsState] = useState<SidebarAppearanceSettings>(
    () => readSidebarAppearanceSettings(localStorage),
  );
  const [backgroundImage, setBackgroundImageBlob] = useState<Blob | null>(null);
  const [backgroundImageUrl, setBackgroundImageUrl] = useState<string | null>(null);
  const hasExplicitBackgroundEffectSettings = useRef(hasStoredBackgroundEffectSettings(localStorage));
  const applied = useRef<PreparedSkin | null>(null);
  const activeSignature = useRef<string | null>(null);
  const generation = useRef(0);
  const mutation = useRef(false);
  const previewPrepared = useRef<PreparedSkin | null>(null);
  const previewGeneration = useRef(0);

  useLayoutEffect(() => {
    document.body.dataset.tronApp = "";
    return () => {
      delete document.body.dataset.tronApp;
    };
  }, []);

  useLayoutEffect(() => {
    document.documentElement.style.setProperty("--tron-sidebar-bg-opacity", `${sidebarAppearanceSettings.opacity}%`);
    document.documentElement.style.setProperty("--tron-sidebar-bg-blur", `${sidebarAppearanceSettings.blur}px`);
  }, [sidebarAppearanceSettings]);

  const stopPreviewSkin = useCallback(() => {
    previewGeneration.current++;
    previewPrepared.current?.dispose();
    previewPrepared.current = null;
    setPreviewSkinId(null);
    applied.current?.apply();
  }, []);

  const setColorMode = useCallback((mode: ColorMode) => {
    if (!["light", "dark", "system"].includes(mode)) return;
    localStorage.setItem(colorModeStorageKey, mode);
    setColorModeState(mode);
  }, []);

  useEffect(() => {
    // Persist the migrated preference before the first palette application.
    localStorage.setItem(colorModeStorageKey, readColorMode());
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const onSystemChange = () => setSystemDark(query.matches);
    const onStorage = (event: StorageEvent) => {
      if (event.key === colorModeStorageKey || event.key === null) setColorModeState(readColorMode());
    };
    onSystemChange();
    query.addEventListener("change", onSystemChange);
    window.addEventListener("storage", onStorage);
    return () => {
      query.removeEventListener("change", onSystemChange);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  useEffect(() => {
    let current = true;
    let prepared: PreparedSkin | undefined;
    stopPreviewSkin();
    const resolved = resolveSkin(activeSkin, resolvedMode);
    const signature = JSON.stringify(resolved);
    if (signature === activeSignature.current && applied.current) return;
    void (async () => {
      try {
        prepared = await prepareSkin(resolved);
        if (!current) { prepared.dispose(); return; }
        const apply = () => {
          if (!current || !prepared) return;
          applied.current?.dispose();
          prepared.apply();
          applied.current = prepared;
          prepared = undefined;
          activeSignature.current = signature;
          flushSync(() => { setDisplayedSkin(activeSkin); setDisplayedMode(resolvedMode); });
          localStorage.setItem("tron.skin", activeSkin.id);
          localStorage.setItem("tron.theme", resolvedMode === "dark" ? "matrix-dark" : "light");
        };
        if (applied.current) await revealTheme(apply);
        else apply();
      } catch (reason) {
        if (current) setError(reason instanceof Error ? reason.message : String(reason));
      } finally { prepared?.dispose(); }
    })();
    return () => { current = false; };
  }, [activeSkin, resolvedMode, stopPreviewSkin]);

  const refreshSkins = useCallback(async () => {
    if (mutation.current) return;
    const request = ++generation.current;
    try {
      const [list, { skin: active }] = await Promise.all([skinApi.list(), skinApi.active()]);
      if (request !== generation.current) return;
      if (!active.variants?.light || !active.variants?.dark) {
        throw new Error("Theme package must include light and dark palettes");
      }
      setSkins(list.filter((skin) => skin.variants?.light && skin.variants?.dark));
      setActiveSkin(active);
      setError(null);
    } catch (reason) {
      if (request === generation.current) setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (status !== "authenticated") { setLoading(false); return; }
    setLoading(true);
    void refreshSkins();
    const stop = skinApi.watch(() => void refreshSkins());
    const onFocus = () => void refreshSkins();
    window.addEventListener("focus", onFocus);
    return () => { generation.current++; stop(); window.removeEventListener("focus", onFocus); };
  }, [status, refreshSkins]);
  useEffect(() => () => { generation.current++; previewPrepared.current?.dispose(); applied.current?.dispose(); applied.current = null; activeSignature.current = null; }, []);

  async function selectSkin(id: string) {
    if (mutation.current || (id === activeSkin.id && id === displayedSkin.id && !previewSkinId)) return;
    if (!skins.some((skin) => skin.id === id)) return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    try {
      const { skin } = await skinApi.activate(id);
      if (request !== generation.current) return;
      setActiveSkin(skin);
      setError(null);
    } finally {
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
  async function installMarketSkin(id: string) {
    if (mutation.current) return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    try {
      const { skin } = await skinApi.installMarket(id);
      if (request !== generation.current) return;
      setSkins((current) => [...current.filter((item) => item.id !== skin.id), skin]);
      await skinApi.activate(skin.id);
      if (request === generation.current) setActiveSkin(skin);
    } finally {
      mutation.current = false;
      setLoading(false);
      if (request === generation.current) void refreshSkins();
    }
  }
  async function uninstallSkin(id: string) {
    if (mutation.current || builtins.some((skin) => skin.id === id)) return;
    mutation.current = true;
    stopPreviewSkin();
    ++generation.current;
    setLoading(true);
    try { await skinApi.uninstall(id); }
    finally { mutation.current = false; setLoading(false); }
    await refreshSkins();
  }
  async function previewSkin(id: string) {
    const skin = skins.find((item) => item.id === id);
    if (!skin) return;
    stopPreviewSkin();
    const request = previewGeneration.current;
    const prepared = await prepareSkin(resolveSkin(skin, resolvedMode));
    if (request !== previewGeneration.current) { prepared.dispose(); return; }
    prepared.apply();
    previewPrepared.current = prepared;
    setPreviewSkinId(id);
  }
  useLayoutEffect(() => {
    applySystemFontSize(fontSize);
    localStorage.setItem(systemFontSizeStorageKey, String(fontSize));
  }, [fontSize]);

  useEffect(() => {
    localStorage.setItem(typingEffectStorageKey, typingEffect);
    localStorage.setItem(typingParticleSettingsStorageKey, JSON.stringify(typingParticleSettings));
    return mountTypingEffect(typingEffect, typingParticleSettings);
  }, [typingEffect, typingParticleSettings]);

  useEffect(() => {
    localStorage.setItem(agentReplyEffectStorageKey, agentReplyEffect);
    localStorage.setItem(agentReplySpeedStorageKey, String(agentReplySpeed));
  }, [agentReplyEffect, agentReplySpeed]);

  useEffect(() => {
    if (hasExplicitBackgroundEffectSettings.current) {
      writeBackgroundEffectSettings(localStorage, backgroundEffectSettings);
    }
  }, [backgroundEffectSettings]);

  useEffect(() => {
    writeBackgroundImageSettings(localStorage, backgroundImageSettings);
  }, [backgroundImageSettings]);

  useEffect(() => {
    let current = true;
    void readBackgroundImage().then((image) => {
      if (current) setBackgroundImageBlob(image);
    }).catch(() => {
      if (current) setBackgroundImageBlob(null);
    });
    return () => { current = false; };
  }, []);

  useEffect(() => {
    if (!backgroundImage) {
      setBackgroundImageUrl(null);
      return;
    }
    const url = URL.createObjectURL(backgroundImage);
    setBackgroundImageUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [backgroundImage]);

  const normalizedBackgroundImageSettings = useMemo(
    () => normalizeBackgroundImageSettings(backgroundImageSettings),
    [backgroundImageSettings],
  );

  const updateBackgroundEffectSettings = useCallback((settings: BackgroundEffectSettings) => {
    hasExplicitBackgroundEffectSettings.current = true;
    setBackgroundEffectSettings(normalizeBackgroundEffectSettings(settings));
  }, []);

  const updateSidebarAppearanceSettings = useCallback((settings: SidebarAppearanceSettings) => {
    try {
      setSidebarAppearanceSettingsState(writeSidebarAppearanceSettings(localStorage, settings));
    } catch {
      setSidebarAppearanceSettingsState(normalizeSidebarAppearanceSettings(settings));
    }
  }, []);

  const setBackgroundImage = useCallback(async (image: File) => {
    await saveBackgroundImage(image);
    setBackgroundImageBlob(image);
    setBackgroundImageSettings((current) => normalizeBackgroundImageSettings({ ...current, fileName: image.name }));
  }, []);

  const clearBackgroundImage = useCallback(async () => {
    await deleteBackgroundImage();
    setBackgroundImageBlob(null);
    setBackgroundImageSettings((current) => normalizeBackgroundImageSettings({ ...current, fileName: null }));
  }, []);

  return <ThemeContext.Provider value={{
    fontSize, setFontSize: (size) => { if (isSystemFontSize(size)) setFontSize(size); },
    typingEffect, setTypingEffect: (effect) => {
      if (["off", "particles", "glow", "ripple"].includes(effect)) setTypingEffect(effect);
    },
    agentReplyEffect,
    setAgentReplyEffect: (effect) => setAgentReplyEffect(effect === "off" ? "off" : "typewriter"),
    agentReplySpeed,
    setAgentReplySpeed: (speed) => {
      if (Number.isFinite(speed)) setAgentReplySpeed(Math.min(240, Math.max(30, speed)));
    },
    typingParticleSettings,
    setTypingParticleSettings: (settings) => setTypingParticleSettings({
      ...settings,
      shape: ["circle", "square", "diamond", "star"].includes(settings.shape) ? settings.shape : defaultTypingParticleSettings.shape,
      count: Math.min(24, Math.max(4, settings.count)),
      size: Math.min(8, Math.max(1, settings.size)),
      speed: Math.min(3, Math.max(0.5, settings.speed)),
    }),
    backgroundEffectSettings,
    setBackgroundEffectSettings: updateBackgroundEffectSettings,
    backgroundImageSettings: normalizedBackgroundImageSettings,
    setBackgroundImageSettings: (settings) => setBackgroundImageSettings(normalizeBackgroundImageSettings(settings)),
    sidebarAppearanceSettings,
    setSidebarAppearanceSettings: updateSidebarAppearanceSettings,
    backgroundImageUrl,
    setBackgroundImage,
    clearBackgroundImage,
    theme: displayedMode === "dark" ? "matrix-dark" : "light", colorMode, setColorMode, resolvedMode: displayedMode, activeSkin: displayedSkin, skins,
    loading, error, refreshSkins, selectSkin, importSkin, installMarketSkin, uninstallSkin, previewSkin, stopPreviewSkin, previewSkinId,
  }}>
    <BackgroundLayer
      effectSettings={backgroundEffectSettings}
      imageSettings={normalizedBackgroundImageSettings}
      imageUrl={backgroundImageUrl}
    />
    {children}
  </ThemeContext.Provider>;
}
export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider.");
  return context;
}
