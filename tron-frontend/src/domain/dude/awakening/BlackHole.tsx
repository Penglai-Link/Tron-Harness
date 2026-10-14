import { useEffect, useRef } from "react";
import * as THREE from "three";
import postFragment from "../../../../../electron/assets/black-hole-post.frag?raw";
import fragment from "../../../../../electron/assets/black-hole.frag?raw";
import "../../../../../electron/assets/black-hole.css";
import "./awakening.css";
const vertex = `varying vec2 vUv; void main(){vUv=uv;gl_Position=vec4(position,1.);}`;
/** Always animated: intentionally independent of the app motion preference. */
export function BlackHole({
  diving = false,
  arriving = false,
  distant = false,
}: {
  diving?: boolean;
  arriving?: boolean;
  distant?: boolean;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const initialArrival = useRef(arriving);
  const initialDistant = useRef(distant);
  const dive = useRef(diving);
  dive.current = diving;
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        canvas: el,
        antialias: false,
        powerPreference: "high-performance",
      });
    } catch {
      return;
    }
    const uniforms = {
      resolution: { value: new THREE.Vector2(1, 1) },
      pointer: { value: new THREE.Vector2() },
      time: { value: 0 },
      plunge: { value: 0 },
      approach: { value: initialArrival.current ? 0 : 1 },
    };
    const material = new THREE.ShaderMaterial({
      vertexShader: vertex,
      fragmentShader: fragment,
      uniforms,
      depthTest: false,
      depthWrite: false,
    });
    const geometry = new THREE.PlaneGeometry(2, 2),
      scene = new THREE.Scene(),
      camera = new THREE.Camera();
    const mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);
    const targets = [0, 1, 2].map(
      () =>
        new THREE.WebGLRenderTarget(1, 1, {
          depthBuffer: false,
          stencilBuffer: false,
          minFilter: THREE.LinearFilter,
          magFilter: THREE.LinearFilter,
        })
    );
    const postUniforms = {
      sceneTexture: { value: targets[0].texture },
      bloomTexture: { value: targets[2].texture },
      resolution: { value: new THREE.Vector2(1, 1) },
      direction: { value: new THREE.Vector2() },
      mode: { value: 0 },
    };
    const postMaterial = new THREE.ShaderMaterial({
      vertexShader: vertex,
      fragmentShader: postFragment,
      uniforms: postUniforms,
      depthTest: false,
      depthWrite: false,
    });
    let frame = 0,
      last = 0,
      elapsed = 0,
      progress = 0,
      disposed = false,
      lost = false,
      slowFrames = 0,
      fastFrames = 0;
    const startedAt = performance.now();
    let diveStartedAt: number | null = null;
    const target = new THREE.Vector2();
    let scale = Math.min(window.devicePixelRatio || 1, 2);
    const resize = () => {
      const { width, height } = el.getBoundingClientRect();
      const ratio = Math.min(
        scale,
        Math.sqrt(8294400 / Math.max(1, width * height))
      );
      renderer.setSize(
        Math.max(1, Math.round(width * ratio)),
        Math.max(1, Math.round(height * ratio)),
        false
      );
      uniforms.resolution.value.set(el.width, el.height);
      targets.forEach((target, index) =>
        target.setSize(
          Math.max(1, Math.round(el.width / (index ? 4 : 1))),
          Math.max(1, Math.round(el.height / (index ? 4 : 1)))
        )
      );
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);
    resize();
    const move = (e: PointerEvent) =>
      target.set(
        (e.clientX / window.innerWidth - 0.5) * 2,
        (0.5 - e.clientY / window.innerHeight) * 2
      );
    const draw = (now: number) => {
      if (disposed || lost || document.hidden) return;
      const dt = last ? Math.min((now - last) / 1000, 0.06) : 0;
      last = now;
      elapsed = (Date.now() % 3600000) / 1000;
      // Sustained pressure may reduce supersampling, but never render below
      // one pixel per CSS pixel. A few slow frames must not permanently blur it.
      if (now - startedAt > 3000 && dt > 0) {
        slowFrames = dt > 0.045 ? slowFrames + 1 : Math.max(0, slowFrames - 2);
        fastFrames = dt < 0.024 ? fastFrames + 1 : 0;
        if (slowFrames > 120 && scale > 1) {
          scale = Math.max(1, scale - 0.25);
          slowFrames = 0;
          resize();
        } else if (
          fastFrames > 600 &&
          scale < Math.min(devicePixelRatio || 1, 2)
        ) {
          scale = Math.min(scale + 0.25, devicePixelRatio || 1, 2);
          fastFrames = 0;
          resize();
        }
      }
      const arrival = initialDistant.current
        ? 0
        : initialArrival.current
        ? Math.min(1, (now - startedAt) / 2400)
        : 1;
      uniforms.approach.value = arrival * arrival * (3 - 2 * arrival);
      if (dive.current && diveStartedAt === null) diveStartedAt = now;
      if (!dive.current) diveStartedAt = null;
      progress =
        diveStartedAt === null ? 0 : Math.min(1, (now - diveStartedAt) / 1800);
      uniforms.plunge.value = progress;
      uniforms.time.value = elapsed;
      uniforms.pointer.value.lerp(target, 0.035);
      try {
        mesh.material = material;
        renderer.setRenderTarget(targets[0]);
        renderer.render(scene, camera);
        mesh.material = postMaterial;
        for (let pass = 0; pass < 3; pass++) {
          const target = pass < 2 ? targets[pass + 1] : null;
          postUniforms.resolution.value.set(
            target?.width ?? el.width,
            target?.height ?? el.height
          );
          postUniforms.mode.value = pass;
          postUniforms.direction.value.set(
            pass === 0 ? 4 / el.width : 0,
            pass === 1 ? 4 / el.height : 0
          );
          postUniforms.sceneTexture.value = targets[pass === 1 ? 1 : 0].texture;
          // Bind a non-attached texture during blur passes to avoid feedback.
          postUniforms.bloomTexture.value = targets[pass === 2 ? 2 : 0].texture;
          renderer.setRenderTarget(target);
          renderer.render(scene, camera);
        }
        el.classList.add("black-hole-ready");
      } catch {
        el.classList.remove("black-hole-ready");
        return;
      }
      frame = requestAnimationFrame(draw);
    };
    const visibility = () => {
      cancelAnimationFrame(frame);
      last = 0;
      if (!document.hidden && !lost) frame = requestAnimationFrame(draw);
    };
    const loss = (e: Event) => {
      e.preventDefault();
      lost = true;
      cancelAnimationFrame(frame);
      el.classList.remove("black-hole-ready");
    };
    const restore = () => {
      lost = false;
      resize();
      visibility();
    };
    window.addEventListener("pointermove", move, { passive: true });
    document.addEventListener("visibilitychange", visibility);
    el.addEventListener("webglcontextlost", loss);
    el.addEventListener("webglcontextrestored", restore);
    frame = requestAnimationFrame(draw);
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener("pointermove", move);
      document.removeEventListener("visibilitychange", visibility);
      el.removeEventListener("webglcontextlost", loss);
      el.removeEventListener("webglcontextrestored", restore);
      geometry.dispose();
      material.dispose();
      postMaterial.dispose();
      targets.forEach((target) => target.dispose());
      renderer.dispose();
    };
  }, []);
  return (
    <div
      className="black-hole-scene"
      data-camera-distance={distant ? "far" : arriving ? "arriving" : "near"}
      aria-hidden="true"
    >
      <div className="black-hole-fallback">
        <i />
        <b />
      </div>
      <canvas ref={canvas} />
    </div>
  );
}
