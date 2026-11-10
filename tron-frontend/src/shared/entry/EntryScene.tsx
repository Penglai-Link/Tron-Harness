import { useEffect, useRef } from "react";
import { useMotion } from "@personai/agent-ui-motion-react";
import { mountEntryArtwork } from "../../../../electron/main/entry-art.mjs";
import "../../../../electron/main/entry-scene.css";

export function EntryBackdrop() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { policy } = useMotion();

  useEffect(() => {
    if (!canvasRef.current) return;
    return mountEntryArtwork(canvasRef.current, {
      animated: policy === "full",
    });
  }, [policy]);

  return (
    <div className="tron-entry-atmosphere" aria-hidden="true">
      <canvas ref={canvasRef} className="tron-entry-weave" />
    </div>
  );
}

export function EntryWordmark() {
  return (
    <h1 className="tron-entry-brand" aria-label="TRON Harness">
      <span aria-hidden="true">TRON</span>
      <span aria-hidden="true">Harness</span>
    </h1>
  );
}

export function EntryLoadingIndicator({ label }: { label: string }) {
  return (
    <div className="tron-entry-progress" role="status" aria-label={label} />
  );
}
