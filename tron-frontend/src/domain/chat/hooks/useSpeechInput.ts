import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { isApiError } from "@/shared/api/httpClient";
import { transcribeAudio } from "../api/speechApi";
import { recordingToWav } from "../model/recordedAudio";

type SpeechState = "idle" | "requesting" | "recording" | "transcribing";
let releaseCurrentRecording: (() => void) | null = null;

export function useSpeechInput({ active, scope, onTranscript }: {
  active: boolean; scope: string; onTranscript: (text: string) => void;
}) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [state, setState] = useState<SpeechState>("idle");
  const [seconds, setSeconds] = useState(0);
  const run = useRef(0);
  const recorder = useRef<MediaRecorder | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const request = useRef<AbortController | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const stateRef = useRef<SpeechState>("idle");
  const latest = useRef({ active, scope, onTranscript, t, toast });
  latest.current = { active, scope, onTranscript, t, toast };
  const updateState = useCallback((next: SpeechState) => { stateRef.current = next; setState(next); }, []);
  const release = useCallback(() => {
    if (timer.current) clearInterval(timer.current);
    timer.current = null;
    stream.current?.getTracks().forEach(track => track.stop());
    stream.current = null;
  }, []);
  const cancel = useCallback(() => {
    run.current++;
    request.current?.abort();
    request.current = null;
    if (recorder.current?.state === "recording") recorder.current.stop();
    recorder.current = null;
    release();
    updateState("idle");
    if (releaseCurrentRecording === cancel) releaseCurrentRecording = null;
  }, [release, updateState]);
  useLayoutEffect(() => cancel, [scope, cancel]);
  useLayoutEffect(() => { if (!active) cancel(); }, [active, cancel]);
  useEffect(() => {
    window.addEventListener("pagehide", cancel);
    return () => window.removeEventListener("pagehide", cancel);
  }, [cancel]);

  const toggle = useCallback(async () => {
    if (stateRef.current === "recording") {
      updateState("transcribing");
      recorder.current?.stop();
      release();
      return;
    }
    if (stateRef.current !== "idle" || !latest.current.active) return;
    releaseCurrentRecording?.();
    releaseCurrentRecording = cancel;
    const generation = ++run.current;
    const originScope = latest.current.scope;
    const isCurrent = () => generation === run.current && latest.current.active && latest.current.scope === originScope;
    updateState("requesting");
    setSeconds(0);
    try {
      if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
        throw new DOMException("unsupported", "NotSupportedError");
      }
      const input = await navigator.mediaDevices.getUserMedia({ audio: true });
      if (!isCurrent()) { input.getTracks().forEach(track => track.stop()); return; }
      stream.current = input;
      const mimeType = ["audio/webm;codecs=opus", "audio/mp4", "audio/ogg;codecs=opus"].find(type => MediaRecorder.isTypeSupported(type));
      const capture = new MediaRecorder(input, mimeType ? { mimeType } : undefined);
      recorder.current = capture;
      const chunks: Blob[] = [];
      let size = 0;
      capture.ondataavailable = event => {
        if (!isCurrent()) return;
        size += event.data.size;
        if (size > 8 * 1024 * 1024) {
          cancel(); latest.current.toast.error(latest.current.t("voiceInput.tooLarge"));
        } else if (event.data.size) chunks.push(event.data);
      };
      capture.onerror = () => {
        if (!isCurrent()) return;
        cancel(); latest.current.toast.error(latest.current.t("voiceInput.failed"));
      };
      capture.onstop = () => {
        if (!isCurrent()) return;
        release();
        updateState("transcribing");
        const controller = new AbortController();
        request.current = controller;
        void (async () => {
          try {
            const audio = await recordingToWav(new Blob(chunks, { type: capture.mimeType }));
            if (!isCurrent()) return;
            const result = await transcribeAudio(audio, controller.signal);
            if (!isCurrent()) return;
            if (result.text.trim()) latest.current.onTranscript(result.text.trim());
            else latest.current.toast.info(latest.current.t("voiceInput.empty"));
          } catch (error) {
            if (isCurrent()) latest.current.toast.error(isApiError(error) ? error.message : latest.current.t("voiceInput.failed"));
          } finally {
            if (isCurrent()) cancel();
          }
        })();
      };
      capture.start(1000);
      updateState("recording");
      const started = Date.now();
      timer.current = setInterval(() => {
        const elapsed = Math.floor((Date.now() - started) / 1000);
        setSeconds(elapsed);
        if (elapsed >= 60 && capture.state === "recording") { updateState("transcribing"); capture.stop(); release(); }
      }, 250);
    } catch (error) {
      if (!isCurrent()) return;
      cancel();
      const name = error instanceof DOMException ? error.name : "";
      latest.current.toast.error(latest.current.t(name === "NotAllowedError" ? "voiceInput.permissionDenied" : name === "NotFoundError" ? "voiceInput.noMicrophone" : name === "NotSupportedError" ? "voiceInput.unsupported" : "voiceInput.failed"));
    }
  }, [cancel, release, updateState]);
  return { state, seconds, toggle, cancel };
}
