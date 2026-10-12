import { useEffect, useRef } from "react";
import { mountBackgroundEffect } from "./backgroundEffectAnimation";
import type { BackgroundEffectSettings, BackgroundImageSettings } from "./backgroundPreferences";

export function BackgroundLayer({
  effectSettings,
  imageSettings,
  imageUrl,
}: {
  effectSettings: BackgroundEffectSettings;
  imageSettings: BackgroundImageSettings;
  imageUrl: string | null;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const hasBackground = Boolean(imageUrl || effectSettings.effect !== "off");

  useEffect(() => {
    if (hasBackground) document.body.dataset.backgroundEffects = "";
    else delete document.body.dataset.backgroundEffects;
    if (imageUrl) document.body.dataset.userBackgroundImage = "";
    else delete document.body.dataset.userBackgroundImage;
    return () => {
      delete document.body.dataset.backgroundEffects;
      delete document.body.dataset.userBackgroundImage;
    };
  }, [hasBackground, imageUrl]);

  useEffect(() => {
    if (!canvas.current) return;
    return mountBackgroundEffect(canvas.current, effectSettings);
  }, [effectSettings]);

  if (!hasBackground) return null;
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-0 overflow-hidden"
      data-appearance-background=""
      inert
    >
      {imageUrl ? (
        <>
          <img
            alt=""
            className="absolute inset-0 size-full"
            src={imageUrl}
            style={{
              filter: imageSettings.blur ? `blur(${imageSettings.blur}px)` : undefined,
              objectFit: imageSettings.fit === "fill" ? "fill" : imageSettings.fit,
              objectPosition: imageSettings.position,
              opacity: imageSettings.opacity,
              transform: imageSettings.blur ? "scale(1.04)" : undefined,
            }}
          />
          <div
            className="absolute inset-0"
            style={{
              backgroundColor: "var(--tron-background-image-overlay-color, #ffffff)",
              opacity: imageSettings.overlayOpacity,
            }}
          />
        </>
      ) : null}
      {effectSettings.effect === "aurora" ? (
        <div className="tron-background-aurora" style={{ opacity: effectSettings.opacity, color: effectSettings.color }}>
          <span style={{ animationDuration: `${18 / effectSettings.speed}s` }} />
          <span style={{ animationDuration: `${23 / effectSettings.speed}s` }} />
          <span style={{ animationDuration: `${27 / effectSettings.speed}s` }} />
        </div>
      ) : effectSettings.effect !== "off" ? (
        <canvas className="absolute inset-0 size-full" ref={canvas} />
      ) : null}
    </div>
  );
}
