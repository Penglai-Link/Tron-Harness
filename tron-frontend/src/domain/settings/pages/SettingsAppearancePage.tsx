import { SkinSettings } from "../components/SkinSettings";
import { BackgroundAppearanceSettings } from "../components/BackgroundAppearanceSettings";
import { MotionSettings } from "../components/MotionSettings";
import { AppEntranceSettings } from "../components/AppEntranceSettings";
import { SidebarAppearanceSettings } from "../components/SidebarAppearanceSettings";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader, SurfaceCard, useTheme } from "@/shared";
import {
  isSystemFontSize,
  maximumSystemFontSize,
  minimumSystemFontSize,
} from "@/shared/lib/systemFontSize";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Item,
  ItemContent,
  ItemTitle,
  ItemDescription,
  ItemActions,
} from "@/components/ui/item";
import { InfoTooltip } from "@/components/info-tooltip";
import { OptionSelect } from "@/components/option-select/OptionSelect";
import type { TypingEffect } from "@/shared/theme/ThemeProvider";
import type { TypingParticleShape, TypingParticleStyle } from "@/shared/theme/typingParticles";

export function SettingsAppearancePage() {
  const { t: shellT } = useTranslation("shell");
  const { t: pageT } = useTranslation("pages");
  const {
    fontSize,
    setFontSize,
    typingEffect,
    setTypingEffect,
    agentReplyEffect,
    setAgentReplyEffect,
    agentReplySpeed,
    setAgentReplySpeed,
  } = useTheme();
  const [draftFontSize, setDraftFontSize] = useState(String(fontSize));

  useEffect(() => {
    setDraftFontSize(String(fontSize));
  }, [fontSize]);

  function handleFontSizeChange(value: string) {
    setDraftFontSize(value);
    const nextFontSize = Number(value);
    if (isSystemFontSize(nextFontSize)) {
      setFontSize(nextFontSize);
    }
  }

  function restoreCurrentFontSize() {
    if (!isSystemFontSize(Number(draftFontSize))) {
      setDraftFontSize(String(fontSize));
    }
  }

  const tips = useMemo(() => {
    return pageT("settingsAppearance.fontSizeError", {
      min: minimumSystemFontSize,
      max: maximumSystemFontSize,
    });
  }, [pageT]);

  return (
    <div className="flex w-full flex-col gap-4">
      <PageHeader title={shellT("settings.items.appearance")} />
      <SkinSettings />
      <SurfaceCard className="p-0">
        <Item size="xs">
          <ItemContent>
            <ItemTitle>
              {pageT("settingsAppearance.fontSize")}
              <InfoTooltip ariaLabel={tips}>{tips}</InfoTooltip>
            </ItemTitle>
            <ItemDescription>
              {pageT("settingsAppearance.fontSizeTips")}
            </ItemDescription>
          </ItemContent>
          <ItemActions>
            <Input
              aria-label={pageT("settingsAppearance.fontSize")}
              max={maximumSystemFontSize}
              min={minimumSystemFontSize}
              step={1}
              type="number"
              value={draftFontSize}
              onBlur={restoreCurrentFontSize}
              onChange={(event) => handleFontSizeChange(event.target.value)}
            />
            <ItemDescription className="flex-shrink-0">px</ItemDescription>
          </ItemActions>
        </Item>
      </SurfaceCard>
      <BackgroundAppearanceSettings />
      <SidebarAppearanceSettings />
      <AppEntranceSettings />
      <MotionSettings />
      <SurfaceCard className="p-0">
        <div className="space-y-2 border-b border-[var(--color-outline-variant)]/50 px-3 pb-3 pt-3">
          <ItemTitle>{pageT("settingsAppearance.typingEffect")}</ItemTitle>
          <div className="flex flex-wrap gap-2" role="group" aria-label={pageT("settingsAppearance.typingEffect")}>
            {[
              { value: "off", label: pageT("settingsAppearance.typingEffectOff") },
              { value: "particles", label: pageT("settingsAppearance.typingEffectParticles") },
              { value: "glow", label: pageT("settingsAppearance.typingEffectGlow") },
              { value: "ripple", label: pageT("settingsAppearance.typingEffectRipple") },
            ].map((option) => (
              <Button
                key={option.value}
                aria-pressed={typingEffect === option.value}
                variant={typingEffect === option.value ? "secondary" : "outline"}
                onClick={() => setTypingEffect(option.value as TypingEffect)}
              >
                {option.label}
              </Button>
            ))}
          </div>
        </div>
        {typingEffect !== "off" ? (
          <>
            {typingEffect === "particles" ? <TypingParticleOptions /> : null}
            <TypingEffectPreview />
          </>
        ) : null}
      </SurfaceCard>
      <SurfaceCard className="p-0">
        <Item size="xs">
          <ItemContent>
            <ItemTitle>{pageT("settingsAppearance.agentReplyEffect")}</ItemTitle>
          </ItemContent>
          <ItemActions>
            <div className="w-40 max-w-full">
              <OptionSelect
                ariaLabel={pageT("settingsAppearance.agentReplyEffect")}
                options={[
                  { value: "off", label: pageT("settingsAppearance.typingEffectOff") },
                  { value: "typewriter", label: pageT("settingsAppearance.agentReplyTypewriter") },
                ]}
                value={agentReplyEffect}
                onChange={(value) => setAgentReplyEffect(value === "off" ? "off" : "typewriter")}
              />
            </div>
          </ItemActions>
        </Item>
        {agentReplyEffect === "typewriter" ? (
          <label className="grid gap-1.5 border-t border-[var(--color-outline-variant)]/50 px-3 pb-3 pt-2 text-xs">
            <span className="flex items-center justify-between gap-2">
              <span>{pageT("settingsAppearance.agentReplySpeed")}</span>
              <span className="tabular-nums text-[var(--color-on-surface-variant)]">
                {agentReplySpeed} {pageT("settingsAppearance.agentReplySpeedUnit")}
              </span>
            </span>
            <input
              aria-label={pageT("settingsAppearance.agentReplySpeed")}
              className="h-5 w-full cursor-pointer accent-[var(--color-primary)]"
              max={240}
              min={30}
              step={10}
              type="range"
              value={agentReplySpeed}
              onChange={(event) => setAgentReplySpeed(event.currentTarget.valueAsNumber)}
            />
          </label>
        ) : null}
      </SurfaceCard>
    </div>
  );
}

function TypingEffectPreview() {
  const { t } = useTranslation("pages");
  const [text, setText] = useState("");
  return (
    <div className="border-t border-[var(--color-outline-variant)]/50 px-3 py-2">
      <Input
        aria-label={t("settingsAppearance.typingEffectPreview")}
        placeholder={t("settingsAppearance.typingEffectPreviewHint")}
        value={text}
        onChange={(event) => setText(event.target.value)}
      />
    </div>
  );
}

function TypingParticleOptions() {
  const { t } = useTranslation("pages");
  const { typingParticleSettings, setTypingParticleSettings } = useTheme();
  const settings = typingParticleSettings;

  function update(patch: Partial<typeof settings>) {
    setTypingParticleSettings({ ...settings, ...patch });
  }

  return (
    <div className="space-y-3 border-t border-[var(--color-outline-variant)]/50 px-3 pb-3 pt-2">
      <div className="grid gap-1.5 sm:max-w-64">
        <div className="text-xs font-medium">{t("settingsAppearance.typingParticleStyle")}</div>
        <OptionSelect
          ariaLabel={t("settingsAppearance.typingParticleStyle")}
          options={[
            { value: "colorful", label: t("settingsAppearance.typingParticleColorful") },
            { value: "mosaic", label: t("settingsAppearance.typingParticleMosaic") },
            { value: "neon", label: t("settingsAppearance.typingParticleNeon") },
            { value: "custom", label: t("settingsAppearance.typingParticleCustom") },
          ]}
          value={settings.style}
          onChange={(value) => update({ style: value as TypingParticleStyle })}
        />
      </div>

      <label className="flex items-center justify-between gap-3 text-sm">
        <span>{t("settingsAppearance.typingParticleColor")}</span>
        <Input
          aria-label={t("settingsAppearance.typingParticleColor")}
          className="h-8 w-14 cursor-pointer p-1"
          type="color"
          value={settings.color}
          onChange={(event) => update({ color: event.target.value })}
        />
      </label>

      {settings.style === "custom" ? (
        <div className="grid gap-1.5 sm:max-w-64">
          <span className="text-sm">{t("settingsAppearance.typingParticleShape")}</span>
          <OptionSelect
            ariaLabel={t("settingsAppearance.typingParticleShape")}
            options={[
              { value: "circle", label: t("settingsAppearance.typingParticleShapeCircle") },
              { value: "square", label: t("settingsAppearance.typingParticleShapeSquare") },
              { value: "diamond", label: t("settingsAppearance.typingParticleShapeDiamond") },
              { value: "star", label: t("settingsAppearance.typingParticleShapeStar") },
            ]}
            value={settings.shape}
            onChange={(value) => update({ shape: value as TypingParticleShape })}
          />
        </div>
      ) : null}

      <div className="grid gap-x-6 gap-y-3 sm:grid-cols-3">
        <ParticleRange
          label={t("settingsAppearance.typingParticleCount")}
          max={24}
          min={4}
          value={settings.count}
          onChange={(value) => update({ count: value })}
        />
        <ParticleRange
          label={t("settingsAppearance.typingParticleSize")}
          max={8}
          min={1}
          step={0.5}
          value={settings.size}
          onChange={(value) => update({ size: value })}
        />
        <ParticleRange
          label={t("settingsAppearance.typingParticleSpeed")}
          max={3}
          min={0.5}
          step={0.1}
          value={settings.speed}
          onChange={(value) => update({ speed: value })}
        />
      </div>
    </div>
  );
}

function ParticleRange({
  label,
  max,
  min,
  onChange,
  step = 1,
  value,
}: {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  step?: number;
  value: number;
}) {
  return (
    <label className="grid min-w-0 gap-1 text-xs">
      <span className="flex items-center justify-between gap-2">
        <span>{label}</span>
        <span className="tabular-nums text-[var(--color-on-surface-variant)]">{value}</span>
      </span>
      <input
        aria-label={label}
        className="h-5 w-full cursor-pointer accent-[var(--color-primary)]"
        max={max}
        min={min}
        step={step}
        type="range"
        value={value}
        onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
      />
    </label>
  );
}
