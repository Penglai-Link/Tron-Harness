import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Image as ImageIcon, Trash2, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { OptionSelect } from "@/components/option-select/OptionSelect";
import { ItemTitle } from "@/components/ui/item";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard, useTheme } from "@/shared";
import type { BackgroundEffect, BackgroundEffectSettings } from "@/shared/theme/backgroundPreferences";

const maxBackgroundImageBytes = 16 * 1024 * 1024;
const allowedImageTypes = new Set(["image/avif", "image/gif", "image/jpeg", "image/png", "image/webp"]);

export function BackgroundAppearanceSettings() {
  const { t } = useTranslation("pages");
  const {
    backgroundEffectSettings,
    setBackgroundEffectSettings,
    backgroundImageSettings,
    setBackgroundImageSettings,
    backgroundImageUrl,
    setBackgroundImage,
    clearBackgroundImage,
  } = useTheme();
  const imageInput = useRef<HTMLInputElement>(null);
  const toast = useToast();
  const [savingImage, setSavingImage] = useState(false);

  function updateEffect(patch: Partial<BackgroundEffectSettings>) {
    setBackgroundEffectSettings({ ...backgroundEffectSettings, ...patch });
  }

  async function handleImageChange(file: File | undefined) {
    if (!file) return;
    if (!allowedImageTypes.has(file.type)) {
      toast.error(t("settingsAppearance.backgroundImageInvalidType"));
      return;
    }
    if (file.size > maxBackgroundImageBytes) {
      toast.error(t("settingsAppearance.backgroundImageTooLarge"));
      return;
    }
    setSavingImage(true);
    try {
      await setBackgroundImage(file);
      toast.success(t("settingsAppearance.backgroundImageSaved"));
    } catch {
      toast.error(t("settingsAppearance.backgroundImageSaveFailed"));
    } finally {
      setSavingImage(false);
    }
  }

  async function removeImage() {
    setSavingImage(true);
    try {
      await clearBackgroundImage();
    } catch {
      toast.error(t("settingsAppearance.backgroundImageSaveFailed"));
    } finally {
      setSavingImage(false);
    }
  }

  return (
    <SurfaceCard className="p-0">
      <section className="space-y-3 border-b border-[var(--color-outline-variant)]/50 p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <ItemTitle>{t("settingsAppearance.backgroundImage")}</ItemTitle>
          <div className="flex items-center gap-2">
            <Button disabled={savingImage} variant="outline" onClick={() => imageInput.current?.click()}>
              <Upload aria-hidden="true" />
              {backgroundImageUrl
                ? t("settingsAppearance.backgroundImageReplace")
                : t("settingsAppearance.backgroundImageChoose")}
            </Button>
            {backgroundImageUrl ? (
              <Button
                aria-label={t("settingsAppearance.backgroundImageRemove")}
                disabled={savingImage}
                size="icon-sm"
                title={t("settingsAppearance.backgroundImageRemove")}
                variant="ghost"
                onClick={() => void removeImage()}
              >
                <Trash2 aria-hidden="true" />
              </Button>
            ) : null}
            <input
              accept="image/avif,image/gif,image/jpeg,image/png,image/webp"
              aria-label={t("settingsAppearance.backgroundImageChoose")}
              className="hidden"
              ref={imageInput}
              type="file"
              onChange={(event) => {
                const file = event.currentTarget.files?.[0];
                event.currentTarget.value = "";
                void handleImageChange(file);
              }}
            />
          </div>
        </div>

        <div className="grid gap-3 sm:grid-cols-[10rem_minmax(0,1fr)]">
          <div className="relative grid h-24 place-items-center overflow-hidden rounded-md border border-[var(--color-outline-variant)]/60 bg-[var(--color-surface-container)]">
            {backgroundImageUrl ? (
              <img
                alt=""
                className="size-full"
                src={backgroundImageUrl}
                style={{
                  filter: backgroundImageSettings.blur ? `blur(${backgroundImageSettings.blur / 2}px)` : undefined,
                  objectFit: backgroundImageSettings.fit === "fill" ? "fill" : backgroundImageSettings.fit,
                  objectPosition: backgroundImageSettings.position,
                  opacity: backgroundImageSettings.opacity,
                }}
              />
            ) : (
              <ImageIcon aria-hidden="true" className="size-6 text-[var(--color-on-surface-variant)]" />
            )}
          </div>

          <div className="space-y-3">
            {backgroundImageSettings.fileName ? (
              <div className="truncate text-xs text-[var(--color-on-surface-variant)]" title={backgroundImageSettings.fileName}>
                {backgroundImageSettings.fileName}
              </div>
            ) : null}
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              <SelectControl
                ariaLabel={t("settingsAppearance.backgroundImageFit")}
                disabled={!backgroundImageUrl}
                label={t("settingsAppearance.backgroundImageFit")}
                options={[
                  { value: "cover", label: t("settingsAppearance.backgroundImageCover") },
                  { value: "contain", label: t("settingsAppearance.backgroundImageContain") },
                  { value: "fill", label: t("settingsAppearance.backgroundImageFill") },
                ]}
                value={backgroundImageSettings.fit}
                onChange={(value) => setBackgroundImageSettings({ ...backgroundImageSettings, fit: value as typeof backgroundImageSettings.fit })}
              />
              <SelectControl
                ariaLabel={t("settingsAppearance.backgroundImagePosition")}
                disabled={!backgroundImageUrl}
                label={t("settingsAppearance.backgroundImagePosition")}
                options={[
                  { value: "center", label: t("settingsAppearance.backgroundImagePositionCenter") },
                  { value: "top", label: t("settingsAppearance.backgroundImagePositionTop") },
                  { value: "bottom", label: t("settingsAppearance.backgroundImagePositionBottom") },
                ]}
                value={backgroundImageSettings.position}
                onChange={(value) => setBackgroundImageSettings({ ...backgroundImageSettings, position: value as typeof backgroundImageSettings.position })}
              />
              <RangeControl
                disabled={!backgroundImageUrl}
                label={t("settingsAppearance.backgroundImageOpacity")}
                max={100}
                min={10}
                step={5}
                value={Math.round(backgroundImageSettings.opacity * 100)}
                format={(value) => `${value}%`}
                onChange={(value) => setBackgroundImageSettings({ ...backgroundImageSettings, opacity: value / 100 })}
              />
              <RangeControl
                disabled={!backgroundImageUrl}
                label={t("settingsAppearance.backgroundImageBlur")}
                max={24}
                min={0}
                step={1}
                value={backgroundImageSettings.blur}
                format={(value) => `${value}px`}
                onChange={(value) => setBackgroundImageSettings({ ...backgroundImageSettings, blur: value })}
              />
            </div>
          </div>
        </div>
      </section>

      <section className="space-y-3 p-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <ItemTitle>{t("settingsAppearance.backgroundEffect")}</ItemTitle>
          <div className="w-48 max-w-full">
            <OptionSelect
              ariaLabel={t("settingsAppearance.backgroundEffect")}
              options={[
                { value: "off", label: t("settingsAppearance.backgroundEffectOff") },
                { value: "particles", label: t("settingsAppearance.backgroundEffectParticles") },
                { value: "snowfall", label: t("settingsAppearance.backgroundEffectSnowfall") },
                { value: "stars", label: t("settingsAppearance.backgroundEffectStars") },
                { value: "aurora", label: t("settingsAppearance.backgroundEffectAurora") },
              ]}
              value={backgroundEffectSettings.effect}
              onChange={(value) => updateEffect({ effect: value as BackgroundEffect })}
            />
          </div>
        </div>
        {backgroundEffectSettings.effect !== "off" ? (
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {backgroundEffectSettings.effect !== "aurora" ? (
              <>
                <RangeControl
                  label={t("settingsAppearance.backgroundEffectCount")}
                  max={400}
                  min={20}
                  step={10}
                  value={backgroundEffectSettings.count}
                  format={(value) => String(value)}
                  onChange={(value) => updateEffect({ count: value })}
                />
                <RangeControl
                  label={t("settingsAppearance.backgroundEffectSize")}
                  max={8}
                  min={0.5}
                  step={0.5}
                  value={backgroundEffectSettings.size}
                  format={(value) => `${value}px`}
                  onChange={(value) => updateEffect({ size: value })}
                />
              </>
            ) : null}
            <label className="flex min-h-9 items-center justify-between gap-2 text-sm">
              <span>{t("settingsAppearance.backgroundEffectColor")}</span>
              <Input
                aria-label={t("settingsAppearance.backgroundEffectColor")}
                className="h-8 w-14 cursor-pointer p-1"
                type="color"
                value={backgroundEffectSettings.color}
                onChange={(event) => updateEffect({ color: event.currentTarget.value })}
              />
            </label>
            <RangeControl
              label={t("settingsAppearance.backgroundEffectOpacity")}
              max={100}
              min={5}
              step={5}
              value={Math.round(backgroundEffectSettings.opacity * 100)}
              format={(value) => `${value}%`}
              onChange={(value) => updateEffect({ opacity: value / 100 })}
            />
            <RangeControl
              label={t("settingsAppearance.backgroundEffectSpeed")}
              max={3}
              min={0.1}
              step={0.1}
              value={backgroundEffectSettings.speed}
              format={(value) => `${value.toFixed(1)}×`}
              onChange={(value) => updateEffect({ speed: value })}
            />
          </div>
        ) : null}
      </section>
    </SurfaceCard>
  );
}

function RangeControl({
  disabled = false,
  format,
  label,
  max,
  min,
  onChange,
  step,
  value,
}: {
  disabled?: boolean;
  format: (value: number) => string;
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  step: number;
  value: number;
}) {
  return (
    <label className={`grid gap-1 text-xs ${disabled ? "opacity-50" : ""}`}>
      <span className="flex items-center justify-between gap-2">
        <span>{label}</span>
        <span className="tabular-nums text-[var(--color-on-surface-variant)]">{format(value)}</span>
      </span>
      <input
        aria-label={label}
        className="h-5 w-full cursor-pointer accent-[var(--color-primary)] disabled:cursor-not-allowed"
        disabled={disabled}
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

function SelectControl({
  ariaLabel,
  disabled = false,
  label,
  onChange,
  options,
  value,
}: {
  ariaLabel: string;
  disabled?: boolean;
  label: string;
  onChange: (value: string) => void;
  options: Array<{ label: string; value: string }>;
  value: string;
}) {
  return (
    <label className={`grid gap-1.5 text-xs ${disabled ? "opacity-50" : ""}`}>
      <span>{label}</span>
      <OptionSelect ariaLabel={ariaLabel} disabled={disabled} options={options} value={value} onChange={onChange} />
    </label>
  );
}
