import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { applySystemFontSize, defaultSystemFontSize, isSystemFontSize, readStoredSystemFontSize, systemFontSizeStorageKey } from "@/shared/lib/systemFontSize";
import { useAuthStore } from "@/store/authStore";
import { skinApi } from "./skinApi";
import { prepareSkin, type PreparedSkin } from "./skinEngine";
import {
  defaultTypingParticleSettings,
  mountTypingEffect,
  type TypingParticleSettings,
  type TypingParticleShape,
  type TypingParticleStyle,
} from "./typingParticles";
import type { Skin } from "./Skin";
import light from "./builtin/light.json";
import matrix from "./builtin/matrix-dark.json";

import legacyParticles from "./migrations/matrix-particles.json";
import { BackgroundLayer } from "./BackgroundLayer";
import {
  defaultBackgroundEffectSettings,
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
const builtins = [light, matrix] as Skin[];
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
  backgroundImageUrl: string | null;
  setBackgroundImage: (image: File) => Promise<void>;
  clearBackgroundImage: () => Promise<void>;
  theme: AppTheme;
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
  const [activeSkin, setActiveSkin] = useState<Skin>(() => localStorage.getItem("tron.theme") === "matrix-dark" ? builtins[1] : builtins[0]);
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

  const stopPreviewSkin = useCallback(() => {
    previewGeneration.current++;
    previewPrepared.current?.dispose();
    previewPrepared.current = null;
    setPreviewSkinId(null);
    applied.current?.apply();
  }, []);

  const inheritLegacyParticleEffect = useCallback((skin: Skin) => {
    if (hasExplicitBackgroundEffectSettings.current) return;
    hasExplicitBackgroundEffectSettings.current = true;
    const color = skin.colors["--color-on-surface"];
    const settings = normalizeBackgroundEffectSettings({
      ...defaultBackgroundEffectSettings,
      effect: "particles",
      color: typeof color === "string" && /^#[\da-f]{6}$/i.test(color)
        ? color
        : defaultBackgroundEffectSettings.color,
    });
    writeBackgroundEffectSettings(localStorage, settings);
    setBackgroundEffectSettings(settings);
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
      const [list, { skin: active }] = await Promise.all([skinApi.list(), skinApi.active()]);
      if (request !== generation.current) return;
      // Keep recovery choices available even when the active skin's assets fail.
      setSkins(list.filter((item) => item.id !== legacyParticles.id));
      let skin = active;
      if (skin.id === legacyParticles.id) {
        inheritLegacyParticleEffect(skin);
        const replacement = list.find((item) => item.id === "matrix-dark") ?? matrix as Skin;
        await skinApi.activate(replacement.id);
        if (request !== generation.current) return;
        skin = replacement;
      }
      if (JSON.stringify(skin) !== activeSignature.current) prepared = await prepareSkin(skin);
      if (request !== generation.current) { prepared?.dispose(); return; }
      if (prepared) { stopPreviewSkin(); commit(skin, prepared); }
      setError(null);
    } catch (reason) {
      if (request === generation.current) setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [commit, inheritLegacyParticleEffect, stopPreviewSkin]);

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
  async function installMarketSkin(id: string) {
    if (mutation.current) return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    let prepared: PreparedSkin | undefined;
    try {
      const { skin } = await skinApi.installMarket(id);
      setSkins((current) => [...current.filter((item) => item.id !== skin.id), skin]);
      prepared = await prepareSkin(skin);
      if (request !== generation.current) return;
      await skinApi.activate(skin.id);
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
  async function uninstallSkin(id: string) {
    if (mutation.current || builtins.some((skin) => skin.id === id)) return;
    mutation.current = true;
    stopPreviewSkin();
    const request = ++generation.current;
    setLoading(true);
    let fallback: PreparedSkin | undefined;
    try {
      // The backend is authoritative: another window may have changed the active
      // skin since this provider last refreshed.
      const { skin: persistedActive } = await skinApi.active();
      if (persistedActive.id === id) {
        fallback = await prepareSkin(builtins[0]);
      } else if (persistedActive.id !== activeSkin.id) {
        fallback = await prepareSkin(persistedActive);
      }
      await skinApi.uninstall(id);
      if (request !== generation.current) return;
      if (fallback) {
        const nextSkin = persistedActive.id === id ? builtins[0] : persistedActive;
        commit(nextSkin, fallback);
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
    backgroundImageUrl,
    setBackgroundImage,
    clearBackgroundImage,
    theme: activeSkin.mode === "Dark" ? "matrix-dark" : "light", activeSkin, skins,
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
